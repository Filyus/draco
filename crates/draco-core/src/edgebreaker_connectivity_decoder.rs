use crate::corner_table::CornerTable;
use crate::geometry_indices::{CornerIndex, FaceIndex, VertexIndex, INVALID_CORNER_INDEX, INVALID_VERTEX_INDEX};
use crate::mesh_edgebreaker_shared::{EdgeFaceName};
use std::collections::HashMap;

pub trait EdgebreakerTraversalDecoder {
    fn decode_symbol(&mut self) -> u32;
    fn decode_start_face_configuration(&mut self) -> bool;
    fn merge_vertices(&mut self, p: VertexIndex, n: VertexIndex);
    fn is_topology_split(&mut self, encoder_symbol_id: i32) -> Option<(EdgeFaceName, i32)>;
    fn on_vertex_created(&mut self, vertex: VertexIndex, symbol_id: i32, corner_index: i32);
    fn on_vertices_swapped(&mut self, v1: VertexIndex, v2: VertexIndex);
    fn on_start_face_decoded(&mut self, corner: CornerIndex);
    fn on_split_symbol_decoded(&mut self, _corner: CornerIndex) {}

    // Matches C++ traversal_decoder_.NewActiveCornerReached(active_corner_stack.back()).
    // Called after each decoded symbol/face to record the traversal order.
    fn new_active_corner_reached(&mut self, _corner: CornerIndex) {}
}

pub struct EdgebreakerConnectivityDecoder {
    pub corner_table: CornerTable,
    pub is_vert_hole: Vec<bool>,
    active_corner_stack: Vec<CornerIndex>,
    topology_split_active_corners: HashMap<i32, CornerIndex>,
    invalid_vertices: Vec<VertexIndex>,
}

impl EdgebreakerConnectivityDecoder {
    pub fn new(num_faces: i32, max_num_vertices: i32) -> Self {
        Self {
            corner_table: CornerTable::new(num_faces as usize),
            is_vert_hole: vec![true; max_num_vertices as usize],
            active_corner_stack: Vec::new(),
            topology_split_active_corners: HashMap::new(),
            invalid_vertices: Vec::new(),
        }
    }

    pub fn decode_connectivity<T: EdgebreakerTraversalDecoder>(
        &mut self,
        num_symbols: i32,
        traversal_decoder: &mut T,
    ) -> Result<i32, String> {
        let max_num_vertices = self.is_vert_hole.len() as i32;
        let mut num_faces = 0;

        for symbol_id in 0..num_symbols {
            let face = FaceIndex(num_faces as u32);
            num_faces += 1;

            let mut check_topology_split = false;
            let symbol = traversal_decoder.decode_symbol();

            if std::env::var("DRACO_CONN_DEBUG").is_ok() {
                eprintln!("CONN_DEBUG: symbol_id={} symbol={} (face={})", symbol_id, symbol, face.0);
            }

            // Internal symbol mapping (see `EdgebreakerSymbol`):
            //   Center = 0, Split = 1, Left = 2, Right = 3, End = 4
            if symbol == 0 { // TOPOLOGY_C
                if self.active_corner_stack.is_empty() {
                    return Err("active_corner_stack empty in TOPOLOGY_C".to_string());
                }

                let corner_a = *self.active_corner_stack.last().unwrap();
                let vertex_x = self.corner_table.vertex(self.corner_table.next(corner_a));
                let corner_b = self.corner_table.next(self.corner_table.left_most_corner(vertex_x));

                if corner_a == corner_b {
                    return Err("corner_a == corner_b in TOPOLOGY_C".to_string());
                }
                if self.corner_table.opposite(corner_a) != INVALID_CORNER_INDEX ||
                   self.corner_table.opposite(corner_b) != INVALID_CORNER_INDEX {
                    return Err("Edge already opposite in TOPOLOGY_C".to_string());
                }

                let corner = CornerIndex(3 * face.0);
                self.set_opposite_corners(corner_a, corner + 1);
                self.set_opposite_corners(corner_b, corner + 2);

                let vert_a_prev = self.corner_table.vertex(self.corner_table.previous(corner_a));
                let vert_b_next = self.corner_table.vertex(self.corner_table.next(corner_b));
                if vertex_x == vert_a_prev || vertex_x == vert_b_next {
                    return Err("Degenerate face in TOPOLOGY_C".to_string());
                }

                self.corner_table.map_corner_to_vertex(corner, vertex_x);
                self.corner_table.map_corner_to_vertex(corner + 1, vert_b_next);
                self.corner_table.map_corner_to_vertex(corner + 2, vert_a_prev);
                self.corner_table.set_left_most_corner(vert_a_prev, corner + 2);

                self.is_vert_hole[vertex_x.0 as usize] = false;
                *self.active_corner_stack.last_mut().unwrap() = corner;
            } else if symbol == 3 || symbol == 2 { // Right or Left
                // Symbol 3 = Right, Symbol 2 = Left.
                if self.active_corner_stack.is_empty() {
                    return Err("active_corner_stack empty in TOPOLOGY_R/L".to_string());
                }
                let corner_a = *self.active_corner_stack.last().unwrap();
                if self.corner_table.opposite(corner_a) != INVALID_CORNER_INDEX {
                    return Err("Edge already opposite in TOPOLOGY_R/L".to_string());
                }

                // This matches C++ `MeshEdgebreakerDecoderImpl::DecodeConnectivity()`:
                // - Right: opp_corner = corner + 2, corner_l = corner + 1, corner_r = corner
                // - Left:  opp_corner = corner + 1, corner_l = corner,     corner_r = corner + 2
                let corner = CornerIndex(3 * face.0);
                let (opp_corner, corner_l, corner_r) = if symbol == 3 {
                    // Right
                    (corner + 2, corner + 1, corner)
                } else {
                    // Left
                    (corner + 1, corner, corner + 2)
                };

                self.set_opposite_corners(opp_corner, corner_a);
                let new_vert_index = self.corner_table.add_new_vertex();
                traversal_decoder.on_vertex_created(new_vert_index, symbol_id, opp_corner.0 as i32);

                if self.corner_table.num_vertices() as i32 > max_num_vertices {
                    return Err("Unexpected number of vertices in TOPOLOGY_R/L".to_string());
                }

                self.corner_table.map_corner_to_vertex(opp_corner, new_vert_index);
                self.corner_table.set_left_most_corner(new_vert_index, opp_corner);

                let vertex_r = self.corner_table.vertex(self.corner_table.previous(corner_a));
                self.corner_table.map_corner_to_vertex(corner_r, vertex_r);
                self.corner_table.set_left_most_corner(vertex_r, corner_r);

                self.corner_table.map_corner_to_vertex(corner_l, self.corner_table.vertex(self.corner_table.next(corner_a)));
                *self.active_corner_stack.last_mut().unwrap() = corner;
                check_topology_split = true;
            } else if symbol == 1 { // TOPOLOGY_S
                if self.active_corner_stack.is_empty() {
                    return Err("active_corner_stack empty in TOPOLOGY_S".to_string());
                }
                let corner_b = self.active_corner_stack.pop().unwrap();

                let decoder_split_symbol_id = symbol_id;
                if let Some(corner_from_map) = self.topology_split_active_corners.get(&decoder_split_symbol_id).cloned() {
                    self.active_corner_stack.push(corner_from_map);
                }

                if self.active_corner_stack.is_empty() {
                    return Err("active_corner_stack empty in TOPOLOGY_S after split retrieval".to_string());
                }
                let corner_a = *self.active_corner_stack.last().unwrap();

                if corner_a == corner_b {
                    return Err("corner_a == corner_b in TOPOLOGY_S".to_string());
                }
                if self.corner_table.opposite(corner_a) != INVALID_CORNER_INDEX ||
                   self.corner_table.opposite(corner_b) != INVALID_CORNER_INDEX {
                    return Err("Edge already opposite in TOPOLOGY_S".to_string());
                }

                let corner = CornerIndex(3 * face.0);
                self.set_opposite_corners(corner_a, corner + 2);
                self.set_opposite_corners(corner_b, corner + 1);

                let vertex_p = self.corner_table.vertex(self.corner_table.previous(corner_a));
                self.corner_table.map_corner_to_vertex(corner, vertex_p);
                self.corner_table.map_corner_to_vertex(corner + 1, self.corner_table.vertex(self.corner_table.next(corner_a)));

                let vert_b_prev = self.corner_table.vertex(self.corner_table.previous(corner_b));
                self.corner_table.map_corner_to_vertex(corner + 2, vert_b_prev);
                self.corner_table.set_left_most_corner(vert_b_prev, corner + 2);

                let mut corner_n = self.corner_table.next(corner_b);
                let vertex_n = self.corner_table.vertex(corner_n);
                
                if vertex_n != vertex_p && vertex_n != INVALID_VERTEX_INDEX {
                     traversal_decoder.merge_vertices(vertex_p, vertex_n);
                     self.corner_table.set_left_most_corner(vertex_p, self.corner_table.left_most_corner(vertex_n));

                     let first_corner = corner_n;
                     while corner_n != INVALID_CORNER_INDEX {
                         self.corner_table.map_corner_to_vertex(corner_n, vertex_p);
                         corner_n = self.corner_table.swing_left(corner_n);
                         if corner_n == first_corner {
                             return Err("Cycle detected in vertex merge".to_string());
                         }
                     }

                     self.corner_table.make_vertex_isolated(vertex_n);
                     self.invalid_vertices.push(vertex_n);
                }
                *self.active_corner_stack.last_mut().unwrap() = corner;
                traversal_decoder.on_split_symbol_decoded(corner);
            } else if symbol == 4 { // TOPOLOGY_E
                let corner = CornerIndex(3 * face.0);
                let v0 = self.corner_table.add_new_vertex();
                let v1 = self.corner_table.add_new_vertex();
                let v2 = self.corner_table.add_new_vertex();

                traversal_decoder.on_vertex_created(v0, symbol_id, corner.0 as i32);
                traversal_decoder.on_vertex_created(v1, symbol_id, (corner.0 + 1) as i32);
                traversal_decoder.on_vertex_created(v2, symbol_id, (corner.0 + 2) as i32);

                if self.corner_table.num_vertices() as i32 > max_num_vertices {
                    return Err("Unexpected number of vertices in TOPOLOGY_E".to_string());
                }

                self.corner_table.map_corner_to_vertex(corner, v0);
                self.corner_table.map_corner_to_vertex(corner + 1, v1);
                self.corner_table.map_corner_to_vertex(corner + 2, v2);

                self.corner_table.set_left_most_corner(v0, corner);
                self.corner_table.set_left_most_corner(v1, corner + 1);
                self.corner_table.set_left_most_corner(v2, corner + 2);

                self.active_corner_stack.push(corner);
                check_topology_split = true;
            } else {
                return Err(format!("Unknown symbol {}", symbol));
            }

            if check_topology_split {
                // encoder_symbol_id in C++ is num_symbols - symbol_id - 1
                // Rust loop symbol_id goes 0..num_symbols
                // so this matches.
                let encoder_symbol_id = num_symbols - symbol_id - 1;
                while let Some((split_edge, encoder_split_symbol_id)) = traversal_decoder.is_topology_split(encoder_symbol_id) {
                    if encoder_split_symbol_id < 0 {
                        return Err("Invalid split symbol id".to_string());
                    }
                    let act_top_corner = *self.active_corner_stack.last().unwrap();
                    let new_active_corner = match split_edge {
                        EdgeFaceName::RightFaceEdge => self.corner_table.next(act_top_corner),
                        EdgeFaceName::LeftFaceEdge => self.corner_table.previous(act_top_corner),
                    };
                    let decoder_split_symbol_id = num_symbols - encoder_split_symbol_id - 1;
                    self.topology_split_active_corners.insert(decoder_split_symbol_id, new_active_corner);
                }
            }

            // Inform the traversal decoder that a new active corner has been reached.
            // This is the decoder-side equivalent of the encoder's corner visitation order
            // and is used for attribute sequencing.
            if let Some(&active_corner) = self.active_corner_stack.last() {
                traversal_decoder.new_active_corner_reached(active_corner);
            } else {
                return Err("active_corner_stack empty after decoding symbol".to_string());
            }
        }

        if self.corner_table.num_vertices() as i32 > max_num_vertices {
            return Err("Unexpected number of vertices after first pass".to_string());
        }

        // Process component roots in LIFO order (matching C++ pop_back())
        while let Some(corner) = self.active_corner_stack.pop() {
            let interior_face = traversal_decoder.decode_start_face_configuration();
            if interior_face {
                if num_faces >= self.corner_table.num_faces() as i32 {
                    return Err("More faces than expected in start face config".to_string());
                }
                let corner_a = corner;
                let vert_n = self.corner_table.vertex(self.corner_table.next(corner_a));
                if self.corner_table.left_most_corner(vert_n) == INVALID_CORNER_INDEX {
                    return Err(format!("Invalid left_most_corner for vert_n={}", vert_n.0));
                }
                
                let corner_b = self.corner_table.next(self.corner_table.left_most_corner(vert_n));
                let vert_x = self.corner_table.vertex(self.corner_table.next(corner_b));
                if self.corner_table.left_most_corner(vert_x) == INVALID_CORNER_INDEX {
                    return Err("Invalid left_most_corner for vert_x".to_string());
                }
                
                let corner_c = self.corner_table.next(self.corner_table.left_most_corner(vert_x));
                let vert_p = self.corner_table.vertex(self.corner_table.next(corner_c));

                let face = FaceIndex(num_faces as u32);
                num_faces += 1;
                let new_corner = CornerIndex(3 * face.0);
                self.set_opposite_corners(new_corner, corner_a);
                self.set_opposite_corners(new_corner + 1, corner_b);
                self.set_opposite_corners(new_corner + 2, corner_c);

                self.corner_table.map_corner_to_vertex(new_corner, vert_x);
                self.corner_table.map_corner_to_vertex(new_corner + 1, vert_p);
                self.corner_table.map_corner_to_vertex(new_corner + 2, vert_n);

                for i in 0..3 {
                    self.is_vert_hole[self.corner_table.vertex(new_corner + i).0 as usize] = false;
                }
                // Pass new_corner directly, matching C++ init_corners_.push_back(new_corner)
                traversal_decoder.on_start_face_decoded(new_corner);
            } else {
                // Boundary case: Pass corner directly, matching C++ init_corners_.push_back(corner)
                traversal_decoder.on_start_face_decoded(corner);
            }
        }

        if num_faces != self.corner_table.num_faces() as i32 {
            return Err("Unexpected number of faces at end".to_string());
        }

        let mut num_vertices = self.corner_table.num_vertices() as i32;
        
        // Compact vertices (remove isolated/invalid ones)
        // Match C++ logic: iterate invalid_vertices (in order added!)
        for invalid_vert in &self.invalid_vertices {
            let invalid_vert = *invalid_vert;
            
            // Find the last valid vertex (src_vert)
            let mut src_vert = VertexIndex(num_vertices as u32 - 1);
            while src_vert.0 > 0 && self.corner_table.left_most_corner(src_vert) == INVALID_CORNER_INDEX {
                num_vertices -= 1;
                if num_vertices == 0 { break; }
                src_vert = VertexIndex(num_vertices as u32 - 1);
            }
            if src_vert < invalid_vert {
                continue;  // No need to swap
            }

            // Remap all corners mapped to src_vert to invalid_vert
            // Use SwingRight traversal (matching C++ VertexCornersIterator)
            let start_corner = self.corner_table.left_most_corner(src_vert);
            if start_corner != INVALID_CORNER_INDEX {
                let mut c = start_corner;
                loop {
                    // Check logic: C++ "if (corner_table_->Vertex(cid) != src_vert) { Error }"
                    if self.corner_table.vertex(c) != src_vert {
                        return Err(format!("Vertex mismatch during compaction: corner {} maps to {} expected {}", c.0, self.corner_table.vertex(c).0, src_vert.0));
                    }
                    self.corner_table.map_corner_to_vertex(c, invalid_vert);
                    c = self.corner_table.swing_right(c);
                    if c == INVALID_CORNER_INDEX || c == start_corner {
                         break;
                    }
                }
            }

            self.corner_table.set_left_most_corner(invalid_vert, self.corner_table.left_most_corner(src_vert));
            traversal_decoder.on_vertices_swapped(invalid_vert, src_vert);
            self.corner_table.make_vertex_isolated(src_vert);
            
            if (invalid_vert.0 as usize) < self.is_vert_hole.len() && (src_vert.0 as usize) < self.is_vert_hole.len() {
                self.is_vert_hole[invalid_vert.0 as usize] = self.is_vert_hole[src_vert.0 as usize];
                self.is_vert_hole[src_vert.0 as usize] = false;
            }

            num_vertices -= 1;
        }

        Ok(num_vertices)
    }

    fn set_opposite_corners(&mut self, c1: CornerIndex, c2: CornerIndex) {
        if c1 != INVALID_CORNER_INDEX {
            self.corner_table.set_opposite(c1, c2);
        }
        if c2 != INVALID_CORNER_INDEX {
            self.corner_table.set_opposite(c2, c1);
        }
    }
}
