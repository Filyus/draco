use crate::mesh::Mesh;
use crate::decoder_buffer::DecoderBuffer;
use crate::status::{Status, DracoError, error_status};
use crate::geometry_indices::{PointIndex, FaceIndex};
use crate::mesh_edgebreaker_shared::{EdgebreakerSymbol, TopologySplitEventData};
use crate::rans_bit_decoder::RAnsBitDecoder;
use std::collections::HashMap;

pub struct MeshEdgebreakerDecoder {
    data_to_corner_map: Option<Vec<u32>>,
    attribute_seam_corners: Vec<Vec<u32>>,
}

impl Default for MeshEdgebreakerDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl MeshEdgebreakerDecoder {
    pub fn new() -> Self {
        Self {
            data_to_corner_map: None,
            attribute_seam_corners: Vec::new(),
        }
    }

    pub fn take_data_to_corner_map(&mut self) -> Option<Vec<u32>> {
        self.data_to_corner_map.take()
    }

    pub fn take_attribute_seam_corners(&mut self) -> Vec<Vec<u32>> {
        std::mem::take(&mut self.attribute_seam_corners)
    }

    pub fn get_attribute_seam_corners(&self, attribute_index: usize) -> Option<&Vec<u32>> {
        self.attribute_seam_corners.get(attribute_index)
    }

    pub fn decode_connectivity(&mut self, in_buffer: &mut DecoderBuffer, out_mesh: &mut Mesh) -> Status {
        self.data_to_corner_map = None;

        let version_major = in_buffer.version_major();
        let version_minor = in_buffer.version_minor();
        let bitstream_version = ((version_major as u16) << 8) | (version_minor as u16);
        
        if bitstream_version >= 0x0102 {
            let traversal_decoder_type = in_buffer.decode_u8().map_err(|_| DracoError::DracoError("Failed to read traversal decoder type".to_string()))?;
            if traversal_decoder_type != 0 {
                return Err(DracoError::DracoError(format!("Unsupported Edgebreaker traversal decoder type: {}", traversal_decoder_type)));
            }
        }

        let mut _num_new_vertices = 0;
        if bitstream_version < 0x0202 {
            if bitstream_version < 0x0200 {
                _num_new_vertices = in_buffer.decode_u32().map_err(|_| DracoError::DracoError("Failed to read num_new_vertices".to_string()))?;
            } else {
                _num_new_vertices = in_buffer.decode_varint().map_err(|_| DracoError::DracoError("Failed to read num_new_vertices".to_string()))? as u32;
            }
        }

        let num_encoded_vertices = if bitstream_version < 0x0200 {
            in_buffer.decode_u32().map_err(|_| DracoError::DracoError("Failed to read num_encoded_vertices".to_string()))?
        } else {
            in_buffer.decode_varint().map_err(|_| DracoError::DracoError("Failed to read num_encoded_vertices".to_string()))? as u32
        };

        let num_faces = if bitstream_version < 0x0200 {
            in_buffer.decode_u32().map_err(|_| DracoError::DracoError("Failed to read num_faces".to_string()))?
        } else {
            in_buffer.decode_varint().map_err(|_| DracoError::DracoError("Failed to read num_faces".to_string()))? as u32
        };

        let num_attribute_data = in_buffer.decode_u8().map_err(|_| DracoError::DracoError("Failed to read attribute data count".to_string()))?;

        out_mesh.set_num_faces(num_faces as usize);
        out_mesh.set_num_points(num_encoded_vertices as usize);

        let num_symbols = if bitstream_version < 0x0200 {
            in_buffer.decode_u32().map_err(|_| DracoError::DracoError("Failed to read symbol count".to_string()))? as usize
        } else {
            in_buffer.decode_varint().map_err(|_| DracoError::DracoError("Failed to read symbol count".to_string()))? as usize
        };

        let num_split_symbols = if bitstream_version < 0x0200 {
            in_buffer.decode_u32().map_err(|_| DracoError::DracoError("Failed to read split symbol count".to_string()))? as usize
        } else {
            in_buffer.decode_varint().map_err(|_| DracoError::DracoError("Failed to read split symbol count".to_string()))? as usize
        };

        // Read hole/topology split events.
        // Draco stores these events inline for v2.2+, but for older streams (<2.2)
        // they are stored after the traversal buffer, and the traversal buffer size
        // is explicitly encoded.
        let (topology_split_data, topology_split_decoded_bytes) = if bitstream_version < 0x0202 {
            let encoded_connectivity_size = if bitstream_version < 0x0200 {
                in_buffer
                    .decode_u32()
                    .map_err(|_| DracoError::DracoError("Failed to read encoded_connectivity_size".to_string()))?
                    as usize
            } else {
                in_buffer
                    .decode_varint()
                    .map_err(|_| DracoError::DracoError("Failed to read encoded_connectivity_size".to_string()))?
                    as usize
            };

            if encoded_connectivity_size == 0 || encoded_connectivity_size > in_buffer.remaining_size() {
                return Err(DracoError::DracoError(
                    "Invalid encoded_connectivity_size".to_string(),
                ));
            }

            // Decode events from a temporary buffer starting at the end of the
            // traversal buffer, while keeping |in_buffer| positioned at the start
            // of the traversal buffer.
            let remaining = in_buffer.remaining_data();
            let events_slice = &remaining[encoded_connectivity_size..];
            let mut event_buffer = DecoderBuffer::new(events_slice);
            event_buffer.set_version(version_major, version_minor);

            let (events, decoded_bytes) =
                Self::decode_hole_and_topology_split_events(&mut event_buffer, bitstream_version)?;
            (events, decoded_bytes)
        } else {
            let events = Self::decode_topology_split_events_inline(in_buffer, bitstream_version)?;
            (events, 0)
        };

        // Validate split data count.
        if topology_split_data.len() > num_split_symbols {
            return Err(error_status(format!(
                "Split event count exceeds split-symbol count (split_symbols={num_split_symbols}, events={})",
                topology_split_data.len()
            )));
        }

        // Read symbol stream (reversed from encoder)
        let symbols = Self::decode_symbol_stream(in_buffer, num_symbols)?;

        // Reconstruct topology.
        // Draco allows up to (num_encoded_vertices + num_split_symbols) vertices during
        // connectivity decoding because split symbols can introduce temporary vertices
        // that are eliminated during deduplication.
        let max_num_vertices = (num_encoded_vertices as usize).saturating_add(num_split_symbols);

        self.reconstruct_mesh(
            &symbols,
            &topology_split_data,
            out_mesh,
            num_faces as usize,
            max_num_vertices,
            num_attribute_data,
            in_buffer,
        )?;

        // For pre-v2.2 streams, the hole/topology split event payload was decoded
        // from a temporary buffer, and the main buffer is now positioned at the
        // start of that payload. Advance it so attribute decoding starts at the
        // correct location.
        if topology_split_decoded_bytes > 0 {
            if topology_split_decoded_bytes > in_buffer.remaining_size() {
                return Err(DracoError::DracoError(
                    "Invalid topology split decoded byte count".to_string(),
                ));
            }
            in_buffer.advance(topology_split_decoded_bytes);
        }

        Ok(())
    }

    fn decode_hole_and_topology_split_events(
        in_buffer: &mut DecoderBuffer,
        bitstream_version: u16,
    ) -> Result<(Vec<TopologySplitEventData>, usize), DracoError> {
        // Matches MeshEdgebreakerDecoderImpl::DecodeHoleAndTopologySplitEvents.
        let num_topology_splits = if bitstream_version < 0x0200 {
            in_buffer
                .decode_u32()
                .map_err(|_| DracoError::DracoError("Failed to read num_topology_splits".to_string()))?
        } else {
            in_buffer
                .decode_varint()
                .map_err(|_| DracoError::DracoError("Failed to read num_topology_splits".to_string()))?
                as u32
        };

        let mut events: Vec<TopologySplitEventData> = Vec::with_capacity(num_topology_splits as usize);
        if num_topology_splits > 0 {
            if bitstream_version < 0x0102 {
                // Legacy (<1.2): absolute IDs + explicit edge byte.
                for _ in 0..num_topology_splits {
                    let split_symbol_id = in_buffer
                        .decode_u32()
                        .map_err(|_| DracoError::DracoError("Failed to read split_symbol_id".to_string()))?;
                    let source_symbol_id = in_buffer
                        .decode_u32()
                        .map_err(|_| DracoError::DracoError("Failed to read source_symbol_id".to_string()))?;
                    let edge_data = in_buffer
                        .decode_u8()
                        .map_err(|_| DracoError::DracoError("Failed to read source_edge byte".to_string()))?;
                    events.push(TopologySplitEventData {
                        split_symbol_id,
                        source_symbol_id,
                        source_edge: if (edge_data & 1) == 0 {
                            crate::mesh_edgebreaker_shared::EdgeFaceName::LeftFaceEdge
                        } else {
                            crate::mesh_edgebreaker_shared::EdgeFaceName::RightFaceEdge
                        },
                    });
                }
            } else {
                // Delta + varint IDs.
                let mut last_source_symbol_id: i32 = 0;
                for _ in 0..num_topology_splits {
                    let delta = in_buffer
                        .decode_varint()
                        .map_err(|_| DracoError::DracoError("Failed to read source symbol delta".to_string()))?
                        as i32;
                    let source_symbol_id = last_source_symbol_id + delta;

                    let split_delta = in_buffer
                        .decode_varint()
                        .map_err(|_| DracoError::DracoError("Failed to read split symbol delta".to_string()))?
                        as i32;
                    if split_delta > source_symbol_id {
                        return Err(DracoError::DracoError(
                            "Invalid split symbol delta".to_string(),
                        ));
                    }
                    let split_symbol_id = source_symbol_id - split_delta;

                    events.push(TopologySplitEventData {
                        split_symbol_id: split_symbol_id as u32,
                        source_symbol_id: source_symbol_id as u32,
                        source_edge: crate::mesh_edgebreaker_shared::EdgeFaceName::LeftFaceEdge,
                    });

                    last_source_symbol_id = source_symbol_id;
                }

                // Split edges are bit-coded; for <2.2 streams the decoder reads 2 bits.
                in_buffer
                    .start_bit_decoding(false)
                    .map_err(|_| DracoError::DracoError("Failed to start bit decoding for split-event source_edge bits".to_string()))?;
                for event in &mut events {
                    let bits = if bitstream_version < 0x0202 { 2 } else { 1 };
                    let edge_data = in_buffer
                        .decode_least_significant_bits32(bits)
                        .map_err(|_| DracoError::DracoError("Failed to read split-event source_edge bits".to_string()))?;
                    event.source_edge = if (edge_data & 1) == 0 {
                        crate::mesh_edgebreaker_shared::EdgeFaceName::LeftFaceEdge
                    } else {
                        crate::mesh_edgebreaker_shared::EdgeFaceName::RightFaceEdge
                    };
                }
                in_buffer.end_bit_decoding();
            }
        }

        // Hole events are present only for older streams (<2.1). We currently
        // decode them to advance the buffer, but full HOLE-symbol topology support
        // is not implemented.
        let mut num_hole_events: u32 = 0;
        if bitstream_version < 0x0201 {
            if bitstream_version < 0x0200 {
                num_hole_events = in_buffer
                    .decode_u32()
                    .map_err(|_| DracoError::DracoError("Failed to read num_hole_events".to_string()))?;
            } else {
                num_hole_events = in_buffer
                    .decode_varint()
                    .map_err(|_| DracoError::DracoError("Failed to read num_hole_events".to_string()))?
                    as u32;
            }
        }

        if num_hole_events > 0 {
            if bitstream_version < 0x0102 {
                for _ in 0..num_hole_events {
                    // Legacy: raw i32 symbol id.
                    let _sym_id: i32 = in_buffer
                        .decode::<i32>()
                        .map_err(|_| DracoError::DracoError("Failed to read hole event".to_string()))?;
                }
            } else {
                // Delta + varint.
                let mut last_symbol_id: i32 = 0;
                for _ in 0..num_hole_events {
                    let delta = in_buffer
                        .decode_varint()
                        .map_err(|_| DracoError::DracoError("Failed to read hole event delta".to_string()))?
                        as i32;
                    let _sym_id = last_symbol_id + delta;
                    last_symbol_id = _sym_id;
                }
            }

            return Err(DracoError::DracoError(
                "Unsupported Edgebreaker hole events in legacy bitstream".to_string(),
            ));
        }

        Ok((events, in_buffer.position()))
    }

    fn decode_topology_split_events_inline(
        in_buffer: &mut DecoderBuffer,
        bitstream_version: u16,
    ) -> Result<Vec<TopologySplitEventData>, DracoError> {
        // Inline event format is only used in v2.2+ streams.
        if bitstream_version < 0x0202 {
            return Ok(Vec::new());
        }

        let num_events = in_buffer
            .decode_varint()
            .map_err(|_| DracoError::DracoError("Failed to read split event count".to_string()))?
            as usize;
        let mut events = Vec::with_capacity(num_events);

        if num_events > 0 {
            let mut last_source_symbol_id: i32 = 0;
            for _ in 0..num_events {
                let delta = in_buffer
                    .decode_varint()
                    .map_err(|_| DracoError::DracoError("Failed to read source symbol delta".to_string()))?
                    as i32;
                let source_symbol_id = last_source_symbol_id + delta;

                let split_delta = in_buffer
                    .decode_varint()
                    .map_err(|_| DracoError::DracoError("Failed to read split symbol delta".to_string()))?
                    as i32;
                let split_symbol_id = source_symbol_id - split_delta;

                events.push(TopologySplitEventData {
                    split_symbol_id: split_symbol_id as u32,
                    source_symbol_id: source_symbol_id as u32,
                    source_edge: crate::mesh_edgebreaker_shared::EdgeFaceName::LeftFaceEdge,
                });

                last_source_symbol_id = source_symbol_id;
            }
        }

        if num_events > 0 {
            in_buffer
                .start_bit_decoding(false)
                .map_err(|_| DracoError::DracoError("Failed to start bit decoding for split-event source_edge bits".to_string()))?;
            for event in &mut events {
                let edge_bit = in_buffer
                    .decode_least_significant_bits32(1)
                    .map_err(|_| DracoError::DracoError("Failed to read split-event source_edge bit".to_string()))?;
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

    // NOTE: Legacy (<2.2) split/hole event decoding is handled by
    // decode_hole_and_topology_split_events().

    fn topology_bit_pattern_to_symbol_id(topology: u32) -> Result<u32, DracoError> {
        // Draco topology bit patterns:
        // C=0, S=1, L=3, R=5, E=7.
        // Map them to our internal symbol IDs: C=0,S=1,L=2,R=3,E=4.
        match topology {
            0 => Ok(EdgebreakerSymbol::Center as u32),
            1 => Ok(EdgebreakerSymbol::Split as u32),
            3 => Ok(EdgebreakerSymbol::Left as u32),
            5 => Ok(EdgebreakerSymbol::Right as u32),
            7 => Ok(EdgebreakerSymbol::End as u32),
            _ => Err(DracoError::DracoError(format!(
                "Invalid Edgebreaker topology bit pattern: {topology}"
            ))),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn reconstruct_mesh(
        &mut self,
        symbols: &[u32],
        topology_split_data: &[TopologySplitEventData],
        mesh: &mut Mesh,
        total_num_faces: usize,
        max_num_vertices: usize,
        num_attribute_data: u8,
        in_buffer: &mut DecoderBuffer,
    ) -> Result<usize, DracoError> {
        if symbols.is_empty() {
            return Ok(0);
        }

        let num_symbols = symbols.len();
        let mut num_decoded_faces = num_symbols;
        let mut corner_table = CornerTable::new(total_num_faces);
        
        // Map Decoder Source Symbol ID -> List of Events
        let mut source_to_split_events: HashMap<u32, Vec<TopologySplitEventData>> = HashMap::new();
        for event in topology_split_data {
            let decoder_source_id = (num_symbols as u32) - event.source_symbol_id - 1;
            source_to_split_events.entry(decoder_source_id).or_default().push(event.clone());
        }

        let mut active_corner_stack: Vec<u32> = Vec::new();
        let mut topology_split_active_corners: HashMap<usize, u32> = HashMap::new();

        let mut next_point_id: u32 = 0;

        // Tracks the corner index at which a vertex was first created.
        // Indexed by the temporary (pre-compaction) vertex id.
        let mut old_vertex_to_corner_map = vec![u32::MAX; max_num_vertices];

        let mut get_next_point = || -> Result<PointIndex, DracoError> {
            if next_point_id as usize >= max_num_vertices {
                return Err(error_status("Unexpected number of decoded vertices"));
            }
            let p = PointIndex(next_point_id);
            next_point_id += 1;
            Ok(p)
        };

        let mut num_components = 0;

        for symbol_id in 0..num_symbols {
            let face_idx = symbol_id;
            let corner = (face_idx * 3) as u32;
            let symbol = EdgebreakerSymbol::from(symbols[symbol_id]);

            let mut check_topology_split = false;

            match symbol {
                // TOPOLOGY_E in Draco C++ reverse decoding.
                EdgebreakerSymbol::End => {
                    // Create three new vertices at the corners of the new face.
                    let v0 = get_next_point()?;
                    let v1 = get_next_point()?;
                    let v2 = get_next_point()?;

                    old_vertex_to_corner_map[v0.0 as usize] = corner;
                    old_vertex_to_corner_map[v1.0 as usize] = corner + 1;
                    old_vertex_to_corner_map[v2.0 as usize] = corner + 2;

                    corner_table.map_corner_to_vertex(corner, v0);
                    corner_table.map_corner_to_vertex(corner + 1, v1);
                    corner_table.map_corner_to_vertex(corner + 2, v2);

                    corner_table.set_left_most_corner(v0, corner);
                    corner_table.set_left_most_corner(v1, corner + 1);
                    corner_table.set_left_most_corner(v2, corner + 2);

                    // Add the tip corner to the active stack.
                    active_corner_stack.push(corner);
                    check_topology_split = true;
                    // Number of components is not equal to number of TOPOLOGY_E
                    // symbols. Components are derived from the remaining active
                    // edge stack during start-face decoding.
                    num_components = num_components.max(1);
                }
                // TOPOLOGY_C in Draco C++ reverse decoding.
                EdgebreakerSymbol::Center => {
                    if active_corner_stack.is_empty() {
                        return Err(error_status("Empty active corner stack on C"));
                    }
                    let corner_a = *active_corner_stack.last().expect("checked non-empty above");

                    let vertex_x = corner_table.get_vertex(corner_table.next(corner_a));
                    let lmc_x = corner_table
                        .left_most_corner(vertex_x)
                        .ok_or_else(|| error_status("Missing left-most corner for vertex_x on C"))?;
                    let corner_b = corner_table.next(lmc_x);

                    if corner_a == corner_b {
                        return Err(error_status("Invalid C symbol: corner_a == corner_b"));
                    }
                    if corner_table.opposite(corner_a).is_some() || corner_table.opposite(corner_b).is_some() {
                        return Err(error_status("Invalid C symbol: matched corner already has opposite"));
                    }

                    // Update opposite corner mappings.
                    corner_table.link(corner_a, corner + 1);
                    corner_table.link(corner_b, corner + 2);

                    // Update vertex mapping.
                    let vert_a_prev = corner_table.get_vertex(corner_table.prev(corner_a));
                    let vert_b_next = corner_table.get_vertex(corner_table.next(corner_b));
                    if vertex_x == vert_a_prev || vertex_x == vert_b_next {
                        return Err(error_status("Invalid C symbol: degenerate face"));
                    }
                    corner_table.map_corner_to_vertex(corner, vertex_x);
                    corner_table.map_corner_to_vertex(corner + 1, vert_b_next);
                    corner_table.map_corner_to_vertex(corner + 2, vert_a_prev);
                    corner_table.set_left_most_corner(vert_a_prev, corner + 2);

                    // Update the corner on the active stack.
                    *active_corner_stack.last_mut().expect("stack non-empty") = corner;
                }
                // TOPOLOGY_R / TOPOLOGY_L in Draco C++ reverse decoding.
                EdgebreakerSymbol::Right | EdgebreakerSymbol::Left => {
                    if active_corner_stack.is_empty() {
                        return Err(error_status("Empty active corner stack on L/R"));
                    }
                    let corner_a = *active_corner_stack.last().expect("checked non-empty above");
                    if corner_table.opposite(corner_a).is_some() {
                        return Err(error_status("Invalid L/R symbol: active corner already has opposite"));
                    }

                    // First corner on the new face is either corner "l" or "r".
                    let (opp_corner, corner_l, corner_r) = if symbol == EdgebreakerSymbol::Right {
                        (corner + 2, corner + 1, corner)
                    } else {
                        (corner + 1, corner, corner + 2)
                    };
                    corner_table.link(opp_corner, corner_a);

                    // New vertex at the opposite corner to corner_a.
                    let new_vert = get_next_point()?;
                    corner_table.map_corner_to_vertex(opp_corner, new_vert);
                    corner_table.set_left_most_corner(new_vert, opp_corner);

                    old_vertex_to_corner_map[new_vert.0 as usize] = opp_corner;

                    let vertex_r = corner_table.get_vertex(corner_table.prev(corner_a));
                    corner_table.map_corner_to_vertex(corner_r, vertex_r);
                    corner_table.set_left_most_corner(vertex_r, corner_r);

                    let vertex_l = corner_table.get_vertex(corner_table.next(corner_a));
                    corner_table.map_corner_to_vertex(corner_l, vertex_l);

                    *active_corner_stack.last_mut().expect("stack non-empty") = corner;
                    check_topology_split = true;
                }
                // TOPOLOGY_S in Draco C++ reverse decoding.
                EdgebreakerSymbol::Split => {
                    if active_corner_stack.is_empty() {
                        return Err(error_status("Empty active corner stack on S"));
                    }
                    let corner_b = *active_corner_stack.last().expect("checked non-empty above");
                    active_corner_stack.pop();

                    // Corner "a" can correspond either to a normal active edge, or to an
                    // edge created from the topology split event.
                    if let Some(&split_corner) = topology_split_active_corners.get(&symbol_id) {
                        active_corner_stack.push(split_corner);
                    }
                    if active_corner_stack.is_empty() {
                        return Err(error_status("Empty active corner stack after topology split on S"));
                    }
                    let corner_a = *active_corner_stack.last().expect("checked non-empty above");

                    if corner_a == corner_b {
                        return Err(error_status("Invalid S symbol: corner_a == corner_b"));
                    }
                    if corner_table.opposite(corner_a).is_some() || corner_table.opposite(corner_b).is_some() {
                        return Err(error_status("Invalid S symbol: matched corner already has opposite"));
                    }

                    // Update opposite corner mapping.
                    corner_table.link(corner_a, corner + 2);
                    corner_table.link(corner_b, corner + 1);

                    // Update vertices.
                    let vertex_p = corner_table.get_vertex(corner_table.prev(corner_a));
                    corner_table.map_corner_to_vertex(corner, vertex_p);
                    corner_table.map_corner_to_vertex(corner + 1, corner_table.get_vertex(corner_table.next(corner_a)));

                    let vert_b_prev = corner_table.get_vertex(corner_table.prev(corner_b));
                    corner_table.map_corner_to_vertex(corner + 2, vert_b_prev);
                    corner_table.set_left_most_corner(vert_b_prev, corner + 2);

                    // Merge vertices p and n.
                    let corner_n = corner_table.next(corner_b);
                    let vertex_n = corner_table.get_vertex(corner_n);

                    // Update left-most corner on the newly merged vertex.
                    if let Some(lmc_n) = corner_table.left_most_corner(vertex_n) {
                        corner_table.set_left_most_corner(vertex_p, lmc_n);
                    }

                    // Update the vertex id at corner "n" and all corners currently
                    // reachable around it. During progressive reconstruction, some
                    // opposite links may not exist yet, so we walk in both directions.
                    {
                        let first_corner = corner_n;
                        let mut act_corner = corner_n;
                        loop {
                            corner_table.map_corner_to_vertex(act_corner, vertex_p);
                            match corner_table.swing_left(act_corner) {
                                Some(c) => {
                                    if c == first_corner {
                                        return Err(error_status(
                                            "Invalid S symbol: reached start again while SwingLeft-walking vertex_n",
                                        ));
                                    }
                                    act_corner = c;
                                }
                                None => break,
                            }
                        }
                    }
                    {
                        let first_corner = corner_n;
                        let mut act_corner = corner_n;
                        loop {
                            corner_table.map_corner_to_vertex(act_corner, vertex_p);
                            match corner_table.swing_right(act_corner) {
                                Some(c) => {
                                    if c == first_corner {
                                        return Err(error_status(
                                            "Invalid S symbol: reached start again while SwingRight-walking vertex_n",
                                        ));
                                    }
                                    act_corner = c;
                                }
                                None => break,
                            }
                        }
                    }

                    // Ensure vertex_n is fully merged away even if some corners are
                    // temporarily disconnected from the swing traversal.
                    for c in &mut corner_table.corners {
                        if c.vertex == vertex_n {
                            c.vertex = vertex_p;
                        }
                    }
                    // Make the old vertex_n isolated.
                    corner_table.make_vertex_isolated(vertex_n);

                    *active_corner_stack.last_mut().expect("stack non-empty") = corner;
                }
                EdgebreakerSymbol::Hole => {
                    // Not expected in current streams.
                }
            }

            if check_topology_split {
                if let Some(events) = source_to_split_events.get(&(symbol_id as u32)) {
                    for event in events {
                        let act_top_corner = *active_corner_stack.last().expect("stack non-empty during topology split");
                        let new_active_corner = if event.source_edge
                            == crate::mesh_edgebreaker_shared::EdgeFaceName::RightFaceEdge
                        {
                            corner_table.next(act_top_corner)
                        } else {
                            corner_table.prev(act_top_corner)
                        };

                        let decoder_split_id = (num_symbols as u32) - event.split_symbol_id - 1;
                        topology_split_active_corners.insert(decoder_split_id as usize, new_active_corner);
                    }
                }
            }
        }

        // Decode start faces and connect them to the remaining active edges.
        // This is required for closed meshes (e.g. torus) where the traversal
        // starts from an interior face that is not represented by symbols.
        if !active_corner_stack.is_empty() {
            let mut start_face_decoder = RAnsBitDecoder::new();
            if !start_face_decoder.start_decoding(in_buffer) {
                return Err(DracoError::DracoError(
                    "Failed to start RAns bit decoding for start faces".to_string(),
                ));
            }

            while let Some(corner_a) = active_corner_stack.pop() {
                let interior_face = start_face_decoder.decode_next_bit();
                if interior_face {
                    if num_decoded_faces >= total_num_faces {
                        start_face_decoder.end_decoding();
                        return Err(error_status("More faces than expected added to the mesh"));
                    }

                    let vert_n = corner_table.get_vertex(corner_table.next(corner_a));
                    let lmc_n = corner_table
                        .left_most_corner(vert_n)
                        .ok_or_else(|| error_status("Missing left-most corner for vert_n on start face"))?;
                    let corner_b = corner_table.next(lmc_n);

                    let vert_x = corner_table.get_vertex(corner_table.next(corner_b));
                    let lmc_x = corner_table
                        .left_most_corner(vert_x)
                        .ok_or_else(|| error_status("Missing left-most corner for vert_x on start face"))?;
                    let corner_c = corner_table.next(lmc_x);

                    if corner_a == corner_b || corner_a == corner_c || corner_b == corner_c {
                        start_face_decoder.end_decoding();
                        return Err(error_status("Invalid start face: matched corners are not distinct"));
                    }
                    if corner_table.opposite(corner_a).is_some()
                        || corner_table.opposite(corner_b).is_some()
                        || corner_table.opposite(corner_c).is_some()
                    {
                        start_face_decoder.end_decoding();
                        return Err(error_status("Invalid start face: corner already has opposite"));
                    }

                    let vert_p = corner_table.get_vertex(corner_table.next(corner_c));

                    let face_idx = num_decoded_faces;
                    num_decoded_faces += 1;
                    let new_corner = (face_idx * 3) as u32;

                    corner_table.link(new_corner, corner_a);
                    corner_table.link(new_corner + 1, corner_b);
                    corner_table.link(new_corner + 2, corner_c);

                    // Map new corners to existing vertices.
                    corner_table.map_corner_to_vertex(new_corner, vert_x);
                    corner_table.map_corner_to_vertex(new_corner + 1, vert_p);
                    corner_table.map_corner_to_vertex(new_corner + 2, vert_n);
                } else {
                    // Exterior configuration: no new face is added.
                }
            }

            start_face_decoder.end_decoding();
        } else {
            // No remaining active corners: still need to consume the rANS bit
            // decoder stream if present. In a valid stream this should be empty.
            // (We leave the buffer untouched here.)
        }

        // Decode attribute seams
        self.attribute_seam_corners.clear();
        for _ in 0..num_attribute_data {
            let mut seam_corners = Vec::new();
            let mut seam_decoder = RAnsBitDecoder::new();
            if !seam_decoder.start_decoding(in_buffer) {
                return Err(DracoError::DracoError("Failed to start seam decoding".to_string()));
            }

            for f in 0..total_num_faces {
                for k in 0..3 {
                    let c = (f * 3 + k) as u32;
                    let opp = corner_table.opposite(c);
                    if opp.is_none() {
                        // Boundary edges are automatically seams
                        seam_corners.push(c);
                        continue;
                    }
                    
                    let opp_val = opp.expect("checked is_some above");
                    let opp_face = (opp_val / 3) as usize;
                    
                    // Only decode seam bit for edges where this face was processed first
                    // (to avoid decoding the same edge twice)
                    if f < opp_face {
                        let is_seam = seam_decoder.decode_next_bit();
                        if is_seam {
                            // Store both corners of the seam edge so that we can
                            // reliably break opposite links in either direction.
                            seam_corners.push(c);
                            seam_corners.push(opp_val);
                        }
                    }
                }
            }
            seam_decoder.end_decoding();
            self.attribute_seam_corners.push(seam_corners);
        }

        if num_decoded_faces != total_num_faces {
            return Err(error_status("Unexpected number of decoded faces"));
        }

        // Compact vertices
        let mut used_point_ids = Vec::new();
        for c in &corner_table.corners {
            used_point_ids.push(c.vertex.0);
        }
        used_point_ids.sort_unstable();
        used_point_ids.dedup();
        
        let mut old_to_new = HashMap::new();
        for (i, &old_id) in used_point_ids.iter().enumerate() {
            old_to_new.insert(old_id, PointIndex(i as u32));
        }

        // Build data_to_corner_map in final (compacted) vertex id order.
        let mut data_to_corner_map = vec![u32::MAX; used_point_ids.len()];
        for (new_id, &old_id) in used_point_ids.iter().enumerate() {
            let corner = old_vertex_to_corner_map
                .get(old_id as usize)
                .copied()
                .unwrap_or(u32::MAX);
            data_to_corner_map[new_id] = corner;
        }
        
        // Update CornerTable
        for c in &mut corner_table.corners {
            if let Some(&new_v) = old_to_new.get(&c.vertex.0) {
                c.vertex = new_v;
            }
        }

        // Rebuild vertex_to_left_most_corner
        corner_table.vertex_to_left_most_corner.clear();
        for (c_idx, c) in corner_table.corners.iter().enumerate() {
            corner_table.vertex_to_left_most_corner.entry(c.vertex).or_insert(c_idx as u32);
        }

        // Copy to mesh
        for i in 0..total_num_faces {
            let (v0, v1, v2) = corner_table.get_face_vertices(i);
            mesh.set_face(FaceIndex(i as u32), [v0, v1, v2]);
        }
        
        mesh.set_num_points(used_point_ids.len());

        // Store mapping for attribute decoding (data id == vertex id for the decoded mesh).
        // Safe because corner indices remain valid after vertex id compaction.
        self.data_to_corner_map = Some(data_to_corner_map);

        Ok(num_components)
    }
    pub fn decode_symbol_stream(in_buffer: &mut DecoderBuffer, num_symbols: usize) -> Result<Vec<u32>, DracoError> {
        if num_symbols == 0 {
            return Ok(Vec::new());
        }

        // Traversal symbols are stored as a size-prefixed bit sequence.
        in_buffer
            .start_bit_decoding(true)
            .map_err(|_| DracoError::DracoError("Failed to start traversal symbol bit decoding".to_string()))?;

        let mut symbols = Vec::with_capacity(num_symbols);
        for _ in 0..num_symbols {
            let first_bit = in_buffer
                .decode_least_significant_bits32(1)
                .map_err(|_| DracoError::DracoError("Failed to read traversal symbol".to_string()))?;
            let topology = if first_bit == 0 {
                0u32
            } else {
                let suffix = in_buffer
                    .decode_least_significant_bits32(2)
                    .map_err(|_| DracoError::DracoError("Failed to read traversal symbol suffix".to_string()))?;
                1u32 | (suffix << 1)
            };
            symbols.push(Self::topology_bit_pattern_to_symbol_id(topology)?);
        }

        // Skip to the end of the traversal symbol bit sequence so subsequent data
        // (start faces, seams) is aligned.
        in_buffer.end_bit_decoding();

        Ok(symbols)
    }
}

struct CornerTable {
    corners: Vec<Corner>,
    vertex_to_left_most_corner: HashMap<PointIndex, u32>,
}

#[derive(Clone, Copy, Debug)]
struct Corner {
    opposite: Option<u32>,
    vertex: PointIndex,
}

impl CornerTable {
    fn new(num_faces: usize) -> Self {
        Self {
            corners: vec![Corner { opposite: None, vertex: PointIndex(0) }; num_faces * 3],
            vertex_to_left_most_corner: HashMap::new(),
        }
    }

    fn set_left_most_corner(&mut self, v: PointIndex, c: u32) {
        self.vertex_to_left_most_corner.insert(v, c);
    }
    
    fn left_most_corner(&self, v: PointIndex) -> Option<u32> {
        self.vertex_to_left_most_corner.get(&v).cloned()
    }

    fn map_corner_to_vertex(&mut self, corner: u32, vertex: PointIndex) {
        self.corners[corner as usize].vertex = vertex;
    }

    fn next(&self, corner: u32) -> u32 {
        if corner % 3 == 2 { corner - 2 } else { corner + 1 }
    }

    fn prev(&self, corner: u32) -> u32 {
        if corner % 3 == 0 { corner + 2 } else { corner - 1 }
    }

    #[allow(dead_code)]
    fn set_face_vertices(&mut self, face_idx: usize, v0: PointIndex, v1: PointIndex, v2: PointIndex) {
        let base = face_idx * 3;
        self.corners[base].vertex = v0;
        self.corners[base + 1].vertex = v1;
        self.corners[base + 2].vertex = v2;
    }

    #[allow(dead_code)]
    fn get_face_vertices(&self, face_idx: usize) -> (PointIndex, PointIndex, PointIndex) {
        let base = face_idx * 3;
        (
            self.corners[base].vertex,
            self.corners[base + 1].vertex,
            self.corners[base + 2].vertex,
        )
    }
    
    fn get_vertex(&self, corner: u32) -> PointIndex {
        self.corners[corner as usize].vertex
    }

    fn link(&mut self, c1: u32, c2: u32) {
        self.corners[c1 as usize].opposite = Some(c2);
        self.corners[c2 as usize].opposite = Some(c1);
    }

    fn opposite(&self, corner: u32) -> Option<u32> {
        self.corners[corner as usize].opposite
    }

    fn swing_left(&self, corner: u32) -> Option<u32> {
        // SwingLeft(c) = Previous(Opposite(Previous(c)))
        let prev = self.prev(corner);
        let opp = self.opposite(prev)?;
        Some(self.prev(opp))
    }

    #[allow(dead_code)]
    fn swing_right(&self, corner: u32) -> Option<u32> {
        // SwingRight(c) = Next(Opposite(Next(c)))
        let next = self.next(corner);
        let opp = self.opposite(next)?;
        Some(self.next(opp))
    }

    fn make_vertex_isolated(&mut self, v: PointIndex) {
        self.vertex_to_left_most_corner.remove(&v);
    }
}
