use crate::mesh::Mesh;
use crate::decoder_buffer::DecoderBuffer;
use crate::status::{Status, DracoError, error_status};
use crate::symbol_encoding::SymbolEncodingOptions;
use crate::geometry_indices::{PointIndex, FaceIndex};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EdgebreakerSymbol {
    Center = 0,
    Split = 1,
    Left = 2,
    Right = 3,
    End = 4,
    Hole = 5,
}

impl From<u32> for EdgebreakerSymbol {
    fn from(v: u32) -> Self {
        match v {
            0 => EdgebreakerSymbol::Center,
            1 => EdgebreakerSymbol::Split,
            2 => EdgebreakerSymbol::Left,
            3 => EdgebreakerSymbol::Right,
            4 => EdgebreakerSymbol::End,
            5 => EdgebreakerSymbol::Hole,
            _ => EdgebreakerSymbol::Hole,
        }
    }
}

pub struct MeshEdgebreakerDecoder {}

impl MeshEdgebreakerDecoder {
    pub fn new() -> Self {
        Self {}
    }

    pub fn decode_connectivity(&mut self, in_buffer: &mut DecoderBuffer, out_mesh: &mut Mesh) -> Status {
        // Edgebreaker connectivity layout (see encoder for full details):
        // num_faces, num_points, num_components, symbol_stream, split_offsets.
        // Read counts
        let num_faces = in_buffer.decode_varint().map_err(|_| DracoError::DracoError("Failed to read face count".to_string()))? as usize;
        let num_points = in_buffer.decode_varint().map_err(|_| DracoError::DracoError("Failed to read point count".to_string()))? as usize;
        let num_components = in_buffer.decode_varint().map_err(|_| DracoError::DracoError("Failed to read component count".to_string()))? as usize;

        out_mesh.set_num_faces(num_faces);
        out_mesh.set_num_points(num_points);

        // Read symbol stream
        let symbols = Self::decode_symbol_stream(in_buffer)?;

        let expected_split_offsets = symbols
            .iter()
            .filter(|&&s| EdgebreakerSymbol::from(s) == EdgebreakerSymbol::Split)
            .count();

        // Read split offsets
        let num_split_offsets = in_buffer.decode_varint().map_err(|_| DracoError::DracoError("Failed to read split offset count".to_string()))? as usize;
        if num_split_offsets != expected_split_offsets {
            return Err(error_status(format!(
                "Split offset count mismatch (expected {expected_split_offsets}, got {num_split_offsets})"
            )));
        }
        let mut split_offsets = Vec::with_capacity(num_split_offsets);
        for _ in 0..num_split_offsets {
            split_offsets.push(in_buffer.decode_varint().map_err(|_| DracoError::DracoError("Failed to read split offset".to_string()))? as u32);
        }

        // Reconstruct topology
        self.reconstruct_mesh(&symbols, &split_offsets, num_components, out_mesh)?;

        Ok(())
    }

    fn reconstruct_mesh(&self, symbols: &[u32], split_offsets: &[u32], num_components: usize, mesh: &mut Mesh) -> Status {
        if symbols.is_empty() {
            return Ok(());
        }

        let mut next_point_id = 0;
        let mut get_next_point = || {
            let p = PointIndex(next_point_id);
            next_point_id += 1;
            p
        };

        let mut face_count: usize = 0;
        let mut symbol_idx = 0;
        let mut split_idx = 0;
        
        for _ in 0..num_components {
            // Initial face (Center)
            if symbol_idx >= symbols.len() {
                break;
            }
            if EdgebreakerSymbol::from(symbols[symbol_idx]) != EdgebreakerSymbol::Center {
                return Err(error_status("First symbol of component must be Center"));
            }
            symbol_idx += 1;

            let v0 = get_next_point();
            let v1 = get_next_point();
            let v2 = get_next_point();
            mesh.set_face(FaceIndex(face_count as u32), [v0, v1, v2]);
            face_count += 1;

            // Boundary tracking
            let mut next_on_boundary: HashMap<PointIndex, PointIndex> = HashMap::new();
            let mut prev_on_boundary: HashMap<PointIndex, PointIndex> = HashMap::new();
            
            next_on_boundary.insert(v0, v1);
            prev_on_boundary.insert(v1, v0);
            next_on_boundary.insert(v1, v2);
            prev_on_boundary.insert(v2, v1);
            next_on_boundary.insert(v2, v0);
            prev_on_boundary.insert(v0, v2);

            // Stack of "active gates" (edges we can attach new triangles to).
            let mut gate_stack: Vec<(PointIndex, PointIndex)> = Vec::new();
            gate_stack.push((v2, v0)); // Left
            gate_stack.push((v0, v1)); // Right

            while symbol_idx < symbols.len() && !gate_stack.is_empty() {
                let symbol = EdgebreakerSymbol::from(symbols[symbol_idx]);
                symbol_idx += 1;

                let (v_start, v_end) = gate_stack.pop().unwrap();

                match symbol {
                    EdgebreakerSymbol::Center => {
                        let v_new = get_next_point();
                        mesh.set_face(FaceIndex(face_count as u32), [v_start, v_end, v_new]);
                        face_count += 1;
                        
                        // Update boundary: replace (v_start, v_end) with (v_start, v_new) and (v_new, v_end)
                        next_on_boundary.insert(v_start, v_new);
                        prev_on_boundary.insert(v_new, v_start);
                        next_on_boundary.insert(v_new, v_end);
                        prev_on_boundary.insert(v_end, v_new);
                        
                        gate_stack.push((v_new, v_end));   // Left
                        gate_stack.push((v_start, v_new)); // Right
                    }
                    EdgebreakerSymbol::Left => {
                        let v_other = *next_on_boundary.get(&v_end).ok_or_else(|| error_status("Boundary error in Left"))?;
                        mesh.set_face(FaceIndex(face_count as u32), [v_start, v_end, v_other]);
                        face_count += 1;
                        
                        // Update boundary: remove (v_start, v_end) and (v_end, v_other), add (v_start, v_other)
                        next_on_boundary.insert(v_start, v_other);
                        prev_on_boundary.insert(v_other, v_start);
                        next_on_boundary.remove(&v_end);
                        prev_on_boundary.remove(&v_end);
                        
                        gate_stack.push((v_start, v_other));
                    }
                    EdgebreakerSymbol::Right => {
                        let v_other = *prev_on_boundary.get(&v_start).ok_or_else(|| error_status("Boundary error in Right"))?;
                        mesh.set_face(FaceIndex(face_count as u32), [v_start, v_end, v_other]);
                        face_count += 1;
                        
                        // Update boundary: remove (v_start, v_end) and (v_other, v_start), add (v_other, v_end)
                        next_on_boundary.insert(v_other, v_end);
                        prev_on_boundary.insert(v_end, v_other);
                        next_on_boundary.remove(&v_start);
                        prev_on_boundary.remove(&v_start);
                        
                        gate_stack.push((v_other, v_end));
                    }
                    EdgebreakerSymbol::End => {
                        let v_other = *next_on_boundary.get(&v_end).ok_or_else(|| error_status("Boundary error in End"))?;
                        mesh.set_face(FaceIndex(face_count as u32), [v_start, v_end, v_other]);
                        face_count += 1;
                        
                        // Update boundary: remove all 3 edges
                        next_on_boundary.remove(&v_start);
                        next_on_boundary.remove(&v_end);
                        next_on_boundary.remove(&v_other);
                        prev_on_boundary.remove(&v_start);
                        prev_on_boundary.remove(&v_end);
                        prev_on_boundary.remove(&v_other);
                    }
                    EdgebreakerSymbol::Split => {
                        if split_idx >= split_offsets.len() {
                            return Err(error_status("Missing split offset for Split symbol"));
                        }
                        let offset = split_offsets[split_idx];
                        split_idx += 1;

                        if offset == 0 {
                            return Err(error_status("Invalid split offset 0"));
                        }

                        // An upper bound to prevent pathological walks or accidental OOB.
                        // `next_on_boundary` stores one outgoing edge per boundary vertex.
                        if offset as usize > next_on_boundary.len() {
                            return Err(error_status("Split offset out of bounds"));
                        }
                        
                        // Find v_other by walking the boundary from v_end
                        let mut v_other = v_end;
                        for _ in 0..offset {
                            v_other = *next_on_boundary.get(&v_other).ok_or_else(|| error_status("Boundary error in Split"))?;
                        }
                        
                        mesh.set_face(FaceIndex(face_count as u32), [v_start, v_end, v_other]);
                        face_count += 1;
                        
                        // Update boundary: split into two loops
                        next_on_boundary.insert(v_start, v_other);
                        prev_on_boundary.insert(v_other, v_start);
                        next_on_boundary.insert(v_other, v_end);
                        prev_on_boundary.insert(v_end, v_other);
                        
                        gate_stack.push((v_other, v_end));   // Left
                        gate_stack.push((v_start, v_other)); // Right
                    }
                    EdgebreakerSymbol::Hole => {
                        // No face here, just pop the gate and do nothing.
                    }
                }
            }
        }

        Ok(())
    }

    /// Decode and return the symbol vector from the buffer.
    pub fn decode_symbol_stream(in_buffer: &mut DecoderBuffer) -> Result<Vec<u32>, DracoError> {
        let num_symbols = in_buffer.decode_varint().map_err(|_| DracoError::DracoError("Failed to read symbol count".to_string()))? as usize;
        let mut symbols = vec![0u32; num_symbols];

        let options = SymbolEncodingOptions::default();
        let ok = crate::symbol_encoding::decode_symbols(num_symbols, 1, &options, in_buffer, &mut symbols);
        if !ok {
            return Err(error_status("Failed to decode Edgebreaker symbol stream"));
        }

        Ok(symbols)
    }
}
