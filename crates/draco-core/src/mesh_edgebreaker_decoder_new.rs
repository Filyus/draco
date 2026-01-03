use crate::mesh::Mesh;
use crate::decoder_buffer::DecoderBuffer;
use crate::status::{Status, DracoError, error_status};
use crate::symbol_encoding::SymbolEncodingOptions;
use crate::geometry_indices::{PointIndex, FaceIndex};
use crate::mesh_edgebreaker_shared::{EdgebreakerSymbol, TopologySplitEventData};
use std::collections::HashMap;

pub struct MeshEdgebreakerDecoder {}

impl MeshEdgebreakerDecoder {
    pub fn new() -> Self {
        Self {}
    }

    pub fn decode_connectivity(&mut self, in_buffer: &mut DecoderBuffer, out_mesh: &mut Mesh) -> Status {
        // C++-compatible Edgebreaker connectivity layout
        let num_vertices = in_buffer.decode_varint().map_err(|_| DracoError::DracoError("Failed to read vertex count".to_string()))? as usize;
        let num_faces = in_buffer.decode_varint().map_err(|_| DracoError::DracoError("Failed to read face count".to_string()))? as usize;
        let _num_attribute_data = in_buffer.decode_u8().map_err(|_| DracoError::DracoError("Failed to read attribute data count".to_string()))?;

        out_mesh.set_num_faces(num_faces);
        out_mesh.set_num_points(num_vertices);

        let num_symbols = in_buffer.decode_varint().map_err(|_| DracoError::DracoError("Failed to read symbol count".to_string()))? as usize;
        let num_split_symbols = in_buffer.decode_varint().map_err(|_| DracoError::DracoError("Failed to read split symbol count".to_string()))? as usize;

        // Read topology split events
        let topology_split_data = Self::decode_topology_split_events(in_buffer)?;

        // Validate split data count
        if topology_split_data.len() != num_split_symbols {
            return Err(error_status(format!(
                "Split event count mismatch (expected {num_split_symbols}, got {})",
                topology_split_data.len()
            )));
        }

        // Read symbol stream (reversed from encoder)
        let symbols = Self::decode_symbol_stream(in_buffer, num_symbols)?;

        // Reconstruct topology
        self.reconstruct_mesh(&symbols, &topology_split_data, out_mesh)?;

        Ok(())
    }

    fn decode_topology_split_events(in_buffer: &mut DecoderBuffer) -> Result<Vec<TopologySplitEventData>, DracoError> {
        let num_events = in_buffer.decode_varint().map_err(|_| DracoError::DracoError("Failed to read split event count".to_string()))? as usize;
        let mut events = Vec::with_capacity(num_events);

        if num_events > 0 {
            // Decode delta-coded source/split symbol IDs
            let mut last_source_symbol_id: i32 = 0;
            for _ in 0..num_events {
                let delta = in_buffer.decode_varint().map_err(|_| DracoError::DracoError("Failed to read source symbol delta".to_string()))? as i32;
                let source_symbol_id = last_source_symbol_id + delta;

                let split_delta = in_buffer.decode_varint().map_err(|_| DracoError::DracoError("Failed to read split symbol delta".to_string()))? as i32;
                let split_symbol_id = source_symbol_id - split_delta;

                events.push(TopologySplitEventData {
                    split_symbol_id: split_symbol_id as u32,
                    source_symbol_id: source_symbol_id as u32,
                    source_edge: crate::mesh_edgebreaker_shared::EdgeFaceName::LeftFaceEdge, // Placeholder
                });

                last_source_symbol_id = source_symbol_id;
            }

            // Decode all source_edge bits
            in_buffer.start_bit_decoding(false).map_err(|_| DracoError::DracoError("Failed to start bit decoding".to_string()))?;
            for event in &mut events {
                let edge_bit = in_buffer.decode_least_significant_bits32(1).map_err(|_| DracoError::DracoError("Failed to read edge bit".to_string()))?;
                event.source_edge = if edge_bit == 0 {
                    crate::mesh_edgebreaker_shared::EdgeFaceName::LeftFaceEdge
                } else {
                    crate::mesh_edgebreaker_shared::EdgeFaceName::RightFaceEdge
                };
            }
            in_buffer.end_bit_decoding();
        }

        Ok(events)
    }

    fn reconstruct_mesh(&self, symbols: &[u32], topology_split_data: &[TopologySplitEventData], mesh: &mut Mesh) -> Status {
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
        
        // Build a map from split_symbol_id to the split event for quick lookup
        let mut topology_split_active_corners: HashMap<usize, (PointIndex, PointIndex)> = HashMap::new();
        for event in topology_split_data {
            // We'll populate the actual corners during decoding when we reach the source symbol
            topology_split_active_corners.insert(event.split_symbol_id as usize, (PointIndex(0), PointIndex(0)));
        }

        // Track active corners/gates
        let mut active_corner_stack: Vec<(PointIndex, PointIndex)> = Vec::new();

        // Process first symbol (must be Center for first component)
        if symbol_idx >= symbols.len() {
            return Err(error_status("Empty symbol stream"));
        }
        if EdgebreakerSymbol::from(symbols[symbol_idx]) != EdgebreakerSymbol::Center {
            return Err(error_status("First symbol must be Center"));
        }
        symbol_idx += 1;

        let v0 = get_next_point();
        let v1 = get_next_point();
        let v2 = get_next_point();
        mesh.set_face(FaceIndex(face_count as u32), [v0, v1, v2]);
        face_count += 1;

        // Initial active edges
        active_corner_stack.push((v2, v0)); // Left
        active_corner_stack.push((v0, v1)); // Right

        // Process remaining symbols
        while symbol_idx < symbols.len() && !active_corner_stack.is_empty() {
            let symbol = EdgebreakerSymbol::from(symbols[symbol_idx]);
            let current_symbol_id = symbol_idx;
            symbol_idx += 1;

            // Check if this is a Split symbol that connects back to an earlier split
            let (v_start, v_end) = if let Some(&gate) = topology_split_active_corners.get(&current_symbol_id) {
                // This is the target of a topology split event
                // Pop the regular active corner and use the split corner instead
                if !active_corner_stack.is_empty() {
                    active_corner_stack.pop();
                }
                gate
            } else {
                if active_corner_stack.is_empty() {
                    return Err(error_status("Empty active corner stack"));
                }
                active_corner_stack.pop().unwrap()
            };

            match symbol {
                EdgebreakerSymbol::Center => {
                    let v_new = get_next_point();
                    mesh.set_face(FaceIndex(face_count as u32), [v_start, v_end, v_new]);
                    face_count += 1;

                    active_corner_stack.push((v_new, v_end));   // Left
                    active_corner_stack.push((v_start, v_new)); // Right
                }
                EdgebreakerSymbol::Right | EdgebreakerSymbol::Left => {
                    let v_new = get_next_point();
                    mesh.set_face(FaceIndex(face_count as u32), [v_start, v_end, v_new]);
                    face_count += 1;

                    if symbol == EdgebreakerSymbol::Right {
                        active_corner_stack.push((v_new, v_end));
                    } else {
                        active_corner_stack.push((v_start, v_new));
                    }

                    // Check for topology split events originating from this symbol
                    for event in topology_split_data {
                        if event.source_symbol_id as usize == current_symbol_id {
                            // Store the active edge for the split symbol to use later
                            let split_gate = match event.source_edge {
                                crate::mesh_edgebreaker_shared::EdgeFaceName::RightFaceEdge => (v_start, v_new),
                                crate::mesh_edgebreaker_shared::EdgeFaceName::LeftFaceEdge => (v_new, v_end),
                            };
                            topology_split_active_corners.insert(event.split_symbol_id as usize, split_gate);
                        }
                    }
                }
                EdgebreakerSymbol::Split => {
                    let v_new = get_next_point();
                    mesh.set_face(FaceIndex(face_count as u32), [v_start, v_end, v_new]);
                    face_count += 1;

                    active_corner_stack.push((v_new, v_end));   // Left
                    active_corner_stack.push((v_start, v_new)); // Right
                }
                EdgebreakerSymbol::End => {
                    let v_new = get_next_point();
                    mesh.set_face(FaceIndex(face_count as u32), [v_start, v_end, v_new]);
                    face_count += 1;

                    // Check for topology split events originating from this symbol
                    for event in topology_split_data {
                        if event.source_symbol_id as usize == current_symbol_id {
                            let split_gate = match event.source_edge {
                                crate::mesh_edgebreaker_shared::EdgeFaceName::RightFaceEdge => (v_start, v_new),
                                crate::mesh_edgebreaker_shared::EdgeFaceName::LeftFaceEdge => (v_new, v_end),
                            };
                            topology_split_active_corners.insert(event.split_symbol_id as usize, split_gate);
                        }
                    }
                    // End closes the active edge, nothing added to stack
                }
                EdgebreakerSymbol::Hole => {
                    // No face created, just pop the gate
                }
            }
        }

        Ok(())
    }

    pub fn decode_symbol_stream(in_buffer: &mut DecoderBuffer, num_symbols: usize) -> Result<Vec<u32>, DracoError> {
        if num_symbols == 0 {
            return Ok(Vec::new());
        }

        let mut symbols = vec![0u32; num_symbols];
        let options = SymbolEncodingOptions::default();
        let ok = crate::symbol_encoding::decode_symbols(num_symbols, 1, &options, in_buffer, &mut symbols);
        if !ok {
            return Err(error_status("Failed to decode Edgebreaker symbol stream"));
        }

        // Symbols were encoded in reverse, so reverse them back
        symbols.reverse();

        Ok(symbols)
    }
}
