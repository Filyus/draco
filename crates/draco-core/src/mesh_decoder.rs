use crate::mesh::Mesh;
use crate::decoder_buffer::DecoderBuffer;
use crate::status::{Status, DracoError};
use crate::compression_config::EncodedGeometryType;
use crate::point_cloud_decoder::PointCloudDecoder;
use crate::geometry_attribute::{PointAttribute, GeometryAttributeType};
use crate::draco_types::DataType;
use crate::sequential_integer_attribute_decoder::SequentialIntegerAttributeDecoder;
use crate::sequential_generic_attribute_decoder::SequentialGenericAttributeDecoder;

use crate::attribute_octahedron_transform::AttributeOctahedronTransform;
use crate::attribute_quantization_transform::AttributeQuantizationTransform;
use crate::attribute_transform::AttributeTransform;
use crate::geometry_indices::AttributeValueIndex;
use crate::corner_table::CornerTable;
use crate::geometry_indices::{PointIndex, FaceIndex, VertexIndex, CornerIndex, INVALID_CORNER_INDEX, INVALID_VERTEX_INDEX};

use crate::mesh_edgebreaker_decoder::MeshEdgebreakerDecoder;
use crate::version::{has_header_flags, uses_varint_encoding, version_less_than, VERSION_FLAGS_INTRODUCED};

pub struct MeshDecoder {
    geometry_type: EncodedGeometryType,
    method: u8,
    flags: u16,
    version_major: u8,
    version_minor: u8,
    corner_table: Option<Box<CornerTable>>,
    edgebreaker_data_to_corner_map: Option<Vec<u32>>,
    edgebreaker_attribute_seam_corners: Vec<Vec<u32>>,
}

impl MeshDecoder {
    pub fn new() -> Self {
        Self {
            geometry_type: EncodedGeometryType::TriangularMesh,
            method: 0,
            flags: 0,
            version_major: 0,
            version_minor: 0,
            corner_table: None,
            edgebreaker_data_to_corner_map: None,
            edgebreaker_attribute_seam_corners: Vec::new(),
        }
    }

    pub fn decode(&mut self, in_buffer: &mut DecoderBuffer, out_mesh: &mut Mesh) -> Status {
        // 1. Decode Header
        self.decode_header(in_buffer)?;
        
        // 2. Decode Metadata
        if (self.flags & 0x8000) != 0 {
            self.decode_metadata(in_buffer)?;
        }

        // 3. Decode Connectivity
        self.decode_connectivity(in_buffer, out_mesh)?;
        
        // 4. Decode Attributes
        self.decode_attributes(in_buffer, out_mesh)
    }

    fn decode_metadata(&self, in_buffer: &mut DecoderBuffer) -> Result<(), DracoError> {
        if version_less_than(self.version_major, self.version_minor, VERSION_FLAGS_INTRODUCED) {
            return Ok(());
        }

        // Draco metadata is encoded using varints and length-prefixed names
        // (see src/draco/metadata/metadata_decoder.cc).
        let num_attribute_metadata = in_buffer
            .decode_varint()
            .map_err(|_| DracoError::DracoError("Failed to read attribute metadata count".to_string()))?
            as u32;
        for _ in 0..num_attribute_metadata {
            let _att_unique_id = in_buffer
                .decode_varint()
                .map_err(|_| DracoError::DracoError("Failed to read attribute unique ID".to_string()))?
                as u32;
            self.skip_metadata(in_buffer)?;
        }
        self.skip_metadata(in_buffer)?; // Geometry metadata
        Ok(())
    }

    fn skip_metadata(&self, in_buffer: &mut DecoderBuffer) -> Result<(), DracoError> {
        let num_entries = in_buffer
            .decode_varint()
            .map_err(|_| DracoError::DracoError("Failed to read metadata entries count".to_string()))?
            as u32;
        for _ in 0..num_entries {
            // Name: u8 length + bytes.
            let name_len = in_buffer
                .decode_u8()
                .map_err(|_| DracoError::DracoError("Failed to read metadata entry name length".to_string()))?
                as usize;
            if in_buffer.remaining_size() < name_len {
                return Err(DracoError::DracoError("Failed to read metadata entry name".to_string()));
            }
            in_buffer.advance(name_len);

            let data_size = in_buffer
                .decode_varint()
                .map_err(|_| DracoError::DracoError("Failed to read metadata entry data size".to_string()))?
                as usize;
            if data_size == 0 {
                return Err(DracoError::DracoError("Invalid metadata entry data size".to_string()));
            }
            if in_buffer.remaining_size() < data_size {
                return Err(DracoError::DracoError("Failed to read metadata entry value".to_string()));
            }
            in_buffer.advance(data_size);
        }

        let num_sub_metadata = in_buffer
            .decode_varint()
            .map_err(|_| DracoError::DracoError("Failed to read sub-metadata count".to_string()))?
            as u32;
        if num_sub_metadata as usize > in_buffer.remaining_size() {
            return Err(DracoError::DracoError("Invalid sub-metadata count".to_string()));
        }
        for _ in 0..num_sub_metadata {
            let name_len = in_buffer
                .decode_u8()
                .map_err(|_| DracoError::DracoError("Failed to read sub-metadata name length".to_string()))?
                as usize;
            if in_buffer.remaining_size() < name_len {
                return Err(DracoError::DracoError("Failed to read sub-metadata name".to_string()));
            }
            in_buffer.advance(name_len);
            self.skip_metadata(in_buffer)?;
        }
        Ok(())
    }

    fn decode_header(&mut self, buffer: &mut DecoderBuffer) -> Status {
        let mut magic = [0u8; 5];
        buffer.decode_bytes(&mut magic)?;
        if &magic != b"DRACO" {
            return Err(DracoError::DracoError("Invalid magic".to_string()));
        }
        
        self.version_major = buffer.decode_u8()?;
        self.version_minor = buffer.decode_u8()?;
        buffer.set_version(self.version_major, self.version_minor);
        
        let g_type = buffer.decode_u8()?;
        self.geometry_type = match g_type {
            0 => EncodedGeometryType::PointCloud,
            1 => EncodedGeometryType::TriangularMesh,
            _ => return Err(DracoError::DracoError("Invalid geometry type".to_string())),
        };
        
        self.method = buffer.decode_u8()?;
        
        if has_header_flags(self.version_major, self.version_minor) {
            self.flags = buffer.decode_u16().map_err(|_| DracoError::DracoError("Failed to decode flags".to_string()))?;
        }
        
        Ok(())
    }

    fn decode_connectivity(&mut self, buffer: &mut DecoderBuffer, mesh: &mut Mesh) -> Status {
        if self.method == 1 {
            let mut eb_decoder = MeshEdgebreakerDecoder::new();
            eb_decoder.decode_connectivity(buffer, mesh)?;

            // Preserve edgebreaker-derived maps for attribute decoding.
            self.edgebreaker_data_to_corner_map = eb_decoder.take_data_to_corner_map();
            self.edgebreaker_attribute_seam_corners = eb_decoder.take_attribute_seam_corners();

            // Initialize CornerTable from decoded faces
            let mut faces = Vec::with_capacity(mesh.num_faces());
            for i in 0..mesh.num_faces() {
                let f = mesh.face(FaceIndex(i as u32));
                faces.push([VertexIndex(f[0].0), VertexIndex(f[1].0), VertexIndex(f[2].0)]);
            }
            
            let mut ct = Box::new(CornerTable::new(0));
            if !ct.init(&faces) {
                 return Err(DracoError::DracoError("Failed to initialize CornerTable after Edgebreaker".to_string()));
            }
            
            // Fix vertex_corners to be First Encountered (matching Encoder's traversal)
            ct.vertex_corners.fill(INVALID_CORNER_INDEX);
            for c in 0..ct.num_corners() {
                let v = ct.vertex(CornerIndex(c as u32));
                if v != INVALID_VERTEX_INDEX {
                    if ct.vertex_corners[v.0 as usize] == INVALID_CORNER_INDEX {
                        ct.vertex_corners[v.0 as usize] = CornerIndex(c as u32);
                    }
                }
            }

            self.corner_table = Some(ct);
        } else {
            // Sequential connectivity encoding
            let (num_faces, num_points) = if !uses_varint_encoding(self.version_major, self.version_minor) {
                let nf = buffer.decode_u32()? as usize;
                let np = buffer.decode_u32()? as usize;
                (nf, np)
            } else {
                let nf = buffer.decode_varint()? as usize;
                let np = buffer.decode_varint()? as usize;
                (nf, np)
            };
            
            mesh.set_num_faces(num_faces);
            mesh.set_num_points(num_points);
            
            if num_faces > 0 && num_points > 0 {
                let connectivity_method = buffer.decode_u8()?;
                let mut indices = vec![0u32; num_faces * 3];
                if connectivity_method == 0 {
                    // Compressed
                    let options = crate::symbol_encoding::SymbolEncodingOptions::default();
                    if !crate::symbol_encoding::decode_symbols(num_faces * 3, 1, &options, buffer, &mut indices) {
                        return Err(DracoError::DracoError("Failed to decode compressed sequential connectivity".to_string()));
                    }
                } else if connectivity_method == 1 {
                    // Raw
                    if num_points < 256 {
                        for i in 0..num_faces * 3 {
                            indices[i] = buffer.decode_u8()? as u32;
                        }
                    } else if num_points < 65536 {
                        for i in 0..num_faces * 3 {
                            indices[i] = buffer.decode_u16()? as u32;
                        }
                    } else {
                        for i in 0..num_faces * 3 {
                            indices[i] = buffer.decode_u32()? as u32;
                        }
                    }
                } else {
                    return Err(DracoError::DracoError(format!("Unsupported sequential connectivity method: {}", connectivity_method)));
                }

                let mut faces = Vec::with_capacity(num_faces);
                for i in 0..num_faces {
                    let v0 = indices[i * 3];
                    let v1 = indices[i * 3 + 1];
                    let v2 = indices[i * 3 + 2];
                    let face = [PointIndex(v0), PointIndex(v1), PointIndex(v2)];
                    mesh.set_face(FaceIndex(i as u32), face);
                    faces.push([VertexIndex(v0), VertexIndex(v1), VertexIndex(v2)]);
                }
                
                // Initialize CornerTable
                let mut ct = Box::new(CornerTable::new(0));
                if !ct.init(&faces) {
                     return Err(DracoError::DracoError("Failed to initialize CornerTable".to_string()));
                }
                self.corner_table = Some(ct);
            }
        }
        
        Ok(())
    }

    fn decode_attributes(&mut self, buffer: &mut DecoderBuffer, mesh: &mut Mesh) -> Status {
        // Both MeshSequentialEncoding and MeshEdgebreakerEncoding use a u8 for the number of
        // attribute decoders.
        let num_attributes_decoders = buffer.decode_u8()? as usize;
        let num_points = mesh.num_points();

        // For Edgebreaker, traversal sequencing is controlled per attribute decoder.
        // We'll derive the correct (point_ids, data_to_corner_map) later for each decoder payload
        // based on its traversal_method.
        let (point_ids, data_to_corner_map): (Vec<PointIndex>, Option<Vec<u32>>) =
            ((0..num_points).map(|i| PointIndex(i as u32)).collect(), None);

        let pc_decoder = PointCloudDecoder::new();
        let bitstream_version: u16 = ((self.version_major as u16) << 8) | (self.version_minor as u16);

        struct PendingQuant {
            att_id: i32,
            portable: PointAttribute,
            transform: AttributeQuantizationTransform,
        }

        struct PendingNormal {
            att_id: i32,
            portable: PointAttribute,
            quantization_bits: u8,
        }

        // (1) Attribute decoder identifiers.
        // For Edgebreaker this ties each decoder payload to attribute connectivity data.
        let mut att_data_id_by_decoder: Vec<u8> = vec![0; num_attributes_decoders];
        let mut traversal_method_by_decoder: Vec<u8> = vec![0; num_attributes_decoders];
        if self.method == 1 {
            for i in 0..num_attributes_decoders {
                att_data_id_by_decoder[i] = buffer.decode_u8()?;
                let _decoder_type = buffer.decode_u8()?;
                traversal_method_by_decoder[i] = buffer.decode_u8()?;
            }
        }

        // (2) Attribute decoder data.
        let mut att_ids_by_decoder: Vec<Vec<i32>> = Vec::with_capacity(num_attributes_decoders);
        let mut decoder_types_by_decoder: Vec<Vec<u8>> = Vec::with_capacity(num_attributes_decoders);

        for _ in 0..num_attributes_decoders {
            let num_attributes_in_decoder: usize = if bitstream_version < 0x0200 {
                buffer.decode_u32()? as usize
            } else {
                buffer.decode_varint()? as usize
            };
            if num_attributes_in_decoder == 0 {
                return Err(DracoError::DracoError("Invalid number of attributes".to_string()));
            }

            let mut att_ids: Vec<i32> = Vec::with_capacity(num_attributes_in_decoder);
            let mut decoder_types: Vec<u8> = Vec::with_capacity(num_attributes_in_decoder);

            for _ in 0..num_attributes_in_decoder {
                let att_type_val = buffer.decode_u8()?;
                let att_type = match att_type_val {
                    0 => GeometryAttributeType::Position,
                    1 => GeometryAttributeType::Normal,
                    2 => GeometryAttributeType::Color,
                    3 => GeometryAttributeType::TexCoord,
                    4 => GeometryAttributeType::Generic,
                    _ => GeometryAttributeType::Invalid,
                };

                let data_type_val = buffer.decode_u8()?;
                let data_type = match data_type_val {
                    1 => DataType::Int8,
                    2 => DataType::Uint8,
                    3 => DataType::Int16,
                    4 => DataType::Uint16,
                    5 => DataType::Int32,
                    6 => DataType::Uint32,
                    7 => DataType::Int64,
                    8 => DataType::Uint64,
                    9 => DataType::Float32,
                    10 => DataType::Float64,
                    11 => DataType::Bool,
                    _ => DataType::Invalid,
                };

                let num_components = buffer.decode_u8()?;
                let normalized = buffer.decode_u8()? != 0;
                let unique_id: u32 = if bitstream_version < 0x0103 {
                    buffer.decode_u16()? as u32
                } else {
                    buffer.decode_varint()? as u32
                };

                let mut att = PointAttribute::new();
                att.init(att_type, num_components, data_type, normalized, num_points);
                att.set_unique_id(unique_id);
                let att_id = mesh.add_attribute(att);
                att_ids.push(att_id);

                if self.method == 1 {
                    let att_mut = mesh.attribute_mut(att_id);
                    att_mut.set_explicit_mapping(num_points);
                    for (i, &pid) in point_ids.iter().enumerate() {
                        att_mut.set_point_map_entry(pid, AttributeValueIndex(i as u32));
                    }
                }
            }

            for _ in 0..num_attributes_in_decoder {
                decoder_types.push(buffer.decode_u8()?);
            }

            att_ids_by_decoder.push(att_ids);
            decoder_types_by_decoder.push(decoder_types);
        }

        // (3) Attribute decoder payloads.
        for dec_i in 0..num_attributes_decoders {
            let att_ids = &att_ids_by_decoder[dec_i];
            let decoder_types = &decoder_types_by_decoder[dec_i];

            // For edgebreaker, build an attribute-specific corner table (seams) if needed.
            // Corner indices remain stable because we only break opposite links.
            let mut attr_corner_table: Option<CornerTable> = None;
            if self.method == 1 {
                let att_data_id = att_data_id_by_decoder[dec_i] as usize;
                if att_data_id < self.edgebreaker_attribute_seam_corners.len() {
                    let seam_corners = &self.edgebreaker_attribute_seam_corners[att_data_id];
                    if !seam_corners.is_empty() {
                        if let Some(base_ct) = self.corner_table.as_deref() {
                            let mut ct = base_ct.clone();
                            for &c_u32 in seam_corners {
                                let c = CornerIndex(c_u32);
                                if c == INVALID_CORNER_INDEX {
                                    continue;
                                }
                                let opp = ct.opposite(c);
                                if opp != INVALID_CORNER_INDEX {
                                    ct.set_opposite(c, INVALID_CORNER_INDEX);
                                    ct.set_opposite(opp, INVALID_CORNER_INDEX);
                                }
                            }
                            attr_corner_table = Some(ct);
                        }
                    }
                }
            }

            // Determine the corner table used for prediction within this decoder.
            // For edgebreaker, seams may split vertex fans and change the effective
            // traversal sequence used by predictors.
            let mut point_ids_for_decoder: Option<Vec<PointIndex>> = None;
            let mut data_to_corner_map_for_decoder: Option<Vec<u32>> = None;
            if self.method == 1 {
                // If we have an attribute-specific seam corner table, recompute vertex
                // corners after breaking opposites so we can derive the correct number
                // of entries for this decoder.
                if let Some(ref mut ct) = attr_corner_table {
                    // Recompute vertex_corners (and potentially split vertices).
                    // Note: compute_vertex_corners may append new vertices.
                    let base_num_vertices = ct.num_vertices();
                    if !ct.compute_vertex_corners(base_num_vertices) {
                        return Err(DracoError::DracoError(
                            "Failed to compute vertex corners for attribute seam table".to_string(),
                        ));
                    }

                    // Build entry->corner and entry->point mappings.
                    let mut map: Vec<u32> = Vec::with_capacity(ct.vertex_corners.len());
                    let mut ids: Vec<PointIndex> = Vec::with_capacity(ct.vertex_corners.len());
                    for &corner in &ct.vertex_corners {
                        if corner == INVALID_CORNER_INDEX {
                            map.push(INVALID_CORNER_INDEX.0);
                            ids.push(PointIndex(0));
                            continue;
                        }
                        map.push(corner.0);
                        let f = (corner.0 / 3) as usize;
                        let k = (corner.0 % 3) as usize;
                        let face = mesh.face(FaceIndex(f as u32));
                        ids.push(face[k]);
                    }
                    point_ids_for_decoder = Some(ids);
                    data_to_corner_map_for_decoder = Some(map);
                }
            }

            let corner_table_for_decoder: Option<&CornerTable> = if let Some(ref ct) = attr_corner_table {
                Some(ct)
            } else {
                self.corner_table.as_deref()
            };

            // For edgebreaker, derive per-decoder traversal sequencing when seams are not
            // applied (per-vertex attributes). This sequencing must match the bitstream
            // traversal_method to keep prediction-scheme side streams (e.g. crease flags)
            // synchronized.
            let mut sequenced_point_ids: Option<Vec<PointIndex>> = None;
            let mut sequenced_data_to_corner_map: Option<Vec<u32>> = None;
            if self.method == 1 && point_ids_for_decoder.is_none() {
                let traversal_method = traversal_method_by_decoder[dec_i];
                let (ids, map) = match traversal_method {
                    0 => self.generate_point_ids_and_corners_dfs(mesh),
                    1 => self.generate_point_ids_and_corners_max_prediction_degree(mesh),
                    _ => self.generate_point_ids_and_corners_dfs(mesh),
                };
                if ids.len() == num_points && map.len() == num_points {
                    // Update per-attribute explicit mappings to match the sequencing.
                    for &att_id in att_ids {
                        let att_mut = mesh.attribute_mut(att_id);
                        att_mut.set_explicit_mapping(num_points);
                        for (i, &pid) in ids.iter().enumerate() {
                            att_mut.set_point_map_entry(pid, AttributeValueIndex(i as u32));
                        }
                    }
                }
                sequenced_point_ids = Some(ids);
                sequenced_data_to_corner_map = Some(map);
            }

            // Choose which point sequence to use for decoding values in this decoder.
            // If seams were applied, we derived a per-decoder point id list (possibly
            // containing repeats). Otherwise, fall back to the mesh-wide sequence.
            let point_ids_for_values: &[PointIndex] = if let Some(ref ids) = point_ids_for_decoder {
                ids
            } else if let Some(ref ids) = sequenced_point_ids {
                ids
            } else {
                &point_ids
            };
            let data_to_corner_map_override_for_values: Option<&[u32]> = if let Some(ref map) = data_to_corner_map_for_decoder {
                Some(map.as_slice())
            } else if let Some(ref map) = sequenced_data_to_corner_map {
                Some(map.as_slice())
            } else {
                data_to_corner_map.as_deref()
            };

            let mut pending_quant: Vec<PendingQuant> = Vec::new();
            let mut pending_normals: Vec<PendingNormal> = Vec::new();

            for (local_i, &att_id) in att_ids.iter().enumerate() {
                let decoder_type = decoder_types[local_i];
                match decoder_type {
                    0 => {
                        let mut att_decoder = SequentialGenericAttributeDecoder::new();
                        att_decoder.init(&pc_decoder, att_id);
                        att_decoder.decode_values(mesh, point_ids_for_values, buffer)?;
                    }
                    1 => {
                        let mut att_decoder = SequentialIntegerAttributeDecoder::new();
                        att_decoder.init(&pc_decoder, att_id);
                        if !att_decoder.decode_values(
                            mesh,
                            point_ids_for_values,
                            buffer,
                            corner_table_for_decoder,
                            data_to_corner_map_override_for_values,
                            None,
                        ) {
                            return Err(DracoError::DracoError("Failed to decode integer attribute values".to_string()));
                        }
                    }
                    2 => {
                        let mut portable = PointAttribute::default();
                        let (original_type, original_num_components) = {
                            let original = mesh.attribute(att_id);
                            (original.attribute_type(), original.num_components())
                        };
                        portable.init(
                            original_type,
                            original_num_components,
                            DataType::Uint32,
                            false,
                            point_ids_for_values.len(),
                        );
                        let mut att_decoder = SequentialIntegerAttributeDecoder::new();
                        att_decoder.init(&pc_decoder, att_id);
                        if !att_decoder.decode_values(
                            mesh,
                            point_ids_for_values,
                            buffer,
                            corner_table_for_decoder,
                            data_to_corner_map_override_for_values,
                            Some(&mut portable),
                        ) {
                            return Err(DracoError::DracoError("Failed to decode quantized portable values".to_string()));
                        }
                        pending_quant.push(PendingQuant {
                            att_id,
                            portable,
                            transform: AttributeQuantizationTransform::new(),
                        });
                    }
                    3 => {
                        let mut portable = PointAttribute::default();
                        portable.init(
                            GeometryAttributeType::Generic,
                            2,
                            DataType::Uint32,
                            false,
                            point_ids_for_values.len(),
                        );
                        let mut att_decoder = SequentialIntegerAttributeDecoder::new();
                        att_decoder.init(&pc_decoder, att_id);
                        if !att_decoder.decode_values(
                            mesh,
                            point_ids_for_values,
                            buffer,
                            corner_table_for_decoder,
                            data_to_corner_map_override_for_values,
                            Some(&mut portable),
                        ) {
                            return Err(DracoError::DracoError("Failed to decode normal portable values".to_string()));
                        }
                        pending_normals.push(PendingNormal {
                            att_id,
                            portable,
                            quantization_bits: 0,
                        });
                    }
                    _ => {
                        return Err(DracoError::DracoError(format!("Unsupported sequential decoder type: {}", decoder_type)));
                    }
                }
            }

            // Decode transform data for all attributes.
            for (local_i, &att_id) in att_ids.iter().enumerate() {
                match decoder_types[local_i] {
                    2 => {
                        let idx = pending_quant
                            .iter()
                            .position(|p| p.att_id == att_id)
                            .ok_or_else(|| DracoError::DracoError("Missing pending quant entry".to_string()))?;
                        let original = mesh.attribute(att_id);
                        if !pending_quant[idx].transform.decode_parameters(original, buffer) {
                            return Err(DracoError::DracoError(
                                "Failed to decode quantization parameters".to_string(),
                            ));
                        }
                    }
                    3 => {
                        let idx = pending_normals
                            .iter()
                            .position(|p| p.att_id == att_id)
                            .ok_or_else(|| DracoError::DracoError("Missing pending normal entry".to_string()))?;
                        let bits = buffer.decode_u8()?;
                        pending_normals[idx].quantization_bits = bits;
                    }
                    _ => {}
                }
            }

            // Apply inverse transforms.
            for q in pending_quant {
                let dst = mesh.attribute_mut(q.att_id);
                if !q.transform.inverse_transform_attribute(&q.portable, dst) {
                    return Err(DracoError::DracoError("Failed to dequantize attribute".to_string()));
                }
            }
            for n in pending_normals {
                let mut oct = AttributeOctahedronTransform::new(-1);
                oct.set_parameters(n.quantization_bits as i32);
                let dst = mesh.attribute_mut(n.att_id);
                if !oct.inverse_transform_attribute(&n.portable, dst) {
                    return Err(DracoError::DracoError("Failed to decode normals".to_string()));
                }
            }
        }

        Ok(())
    }

    fn generate_point_ids_and_corners_dfs(&self, _mesh: &Mesh) -> (Vec<PointIndex>, Vec<u32>) {
        let corner_table = self.corner_table.as_ref().unwrap();
        let num_vertices = corner_table.num_vertices();
        let num_faces = corner_table.num_faces();

        let mut point_ids = Vec::with_capacity(num_vertices);
        let mut data_to_corner_map = Vec::with_capacity(num_vertices);
        let mut visited_vertices = vec![false; num_vertices];
        let mut visited_faces = vec![false; num_faces];

        let mut visit_vertex = |v: VertexIndex, c: CornerIndex, point_ids: &mut Vec<PointIndex>, visited_vertices: &mut [bool]| {
            if v == INVALID_VERTEX_INDEX {
                return;
            }
            let vi = v.0 as usize;
            if vi >= visited_vertices.len() {
                return;
            }
            if !visited_vertices[vi] {
                visited_vertices[vi] = true;
                point_ids.push(PointIndex(v.0));
                data_to_corner_map.push(c.0);
            }
        };

        let mut traverse_from_corner = |start_corner: CornerIndex,
                        point_ids: &mut Vec<PointIndex>,
                        visited_vertices: &mut Vec<bool>,
                        visited_faces: &mut Vec<bool>| {
            let start_face = corner_table.face(start_corner);
            if start_face == crate::geometry_indices::INVALID_FACE_INDEX {
                return;
            }
            if visited_faces[start_face.0 as usize] {
                return;
            }

            let mut corner_stack: Vec<CornerIndex> = Vec::new();
            corner_stack.push(start_corner);

            // Pre-visit next and prev vertices for the first face.
            let next_c = corner_table.next(start_corner);
            let prev_c = corner_table.previous(start_corner);
            let next_v = corner_table.vertex(next_c);
            let prev_v = corner_table.vertex(prev_c);
            visit_vertex(next_v, next_c, point_ids, visited_vertices);
            visit_vertex(prev_v, prev_c, point_ids, visited_vertices);

            while let Some(&corner_id) = corner_stack.last() {
                let face_id = corner_table.face(corner_id);
                if corner_id == INVALID_CORNER_INDEX || face_id == crate::geometry_indices::INVALID_FACE_INDEX || visited_faces[face_id.0 as usize] {
                    corner_stack.pop();
                    continue;
                }

                let mut corner_id = corner_id;
                let mut face_id = face_id;
                loop {
                    visited_faces[face_id.0 as usize] = true;

                    let vert_id = corner_table.vertex(corner_id);
                    if vert_id == INVALID_VERTEX_INDEX {
                        break;
                    }
                    if !visited_vertices[vert_id.0 as usize] {
                        let on_boundary = self.is_vertex_on_boundary(corner_table, vert_id);
                        visit_vertex(vert_id, corner_id, point_ids, visited_vertices);
                        if !on_boundary {
                            corner_id = corner_table.right_corner(corner_id);
                            if corner_id == INVALID_CORNER_INDEX {
                                break;
                            }
                            face_id = corner_table.face(corner_id);
                            if face_id == crate::geometry_indices::INVALID_FACE_INDEX {
                                break;
                            }
                            continue;
                        }
                    }

                    // Vertex already visited or boundary: try neighboring faces.
                    let right_corner_id = corner_table.right_corner(corner_id);
                    let left_corner_id = corner_table.left_corner(corner_id);

                    let right_face_id = if right_corner_id == INVALID_CORNER_INDEX {
                        crate::geometry_indices::INVALID_FACE_INDEX
                    } else {
                        corner_table.face(right_corner_id)
                    };
                    let left_face_id = if left_corner_id == INVALID_CORNER_INDEX {
                        crate::geometry_indices::INVALID_FACE_INDEX
                    } else {
                        corner_table.face(left_corner_id)
                    };

                    let right_visited = right_face_id == crate::geometry_indices::INVALID_FACE_INDEX
                        || visited_faces[right_face_id.0 as usize];
                    let left_visited = left_face_id == crate::geometry_indices::INVALID_FACE_INDEX
                        || visited_faces[left_face_id.0 as usize];

                    if right_visited {
                        if left_visited {
                            corner_stack.pop();
                            break;
                        } else {
                            corner_id = left_corner_id;
                            face_id = left_face_id;
                        }
                    } else {
                        if left_visited {
                            corner_id = right_corner_id;
                            face_id = right_face_id;
                        } else {
                            // Split traversal.
                            *corner_stack.last_mut().unwrap() = left_corner_id;
                            corner_stack.push(right_corner_id);
                            break;
                        }
                    }
                }
            }
        };

        // Traverse all components in face order.
        for f in 0..num_faces {
            if visited_faces[f] {
                continue;
            }
            let first_corner = corner_table.first_corner(FaceIndex(f as u32));
            traverse_from_corner(first_corner, &mut point_ids, &mut visited_vertices, &mut visited_faces);
        }

        // Add any remaining isolated vertices.
        for i in 0..num_vertices {
            if !visited_vertices[i] {
                point_ids.push(PointIndex(i as u32));
                let c = corner_table.left_most_corner(VertexIndex(i as u32));
                data_to_corner_map.push(c.0);
            }
        }

        (point_ids, data_to_corner_map)
    }

    fn generate_point_ids_and_corners_max_prediction_degree(
        &self,
        _mesh: &Mesh,
    ) -> (Vec<PointIndex>, Vec<u32>) {
        // Matches C++ MaxPredictionDegreeTraverser (MESH_TRAVERSAL_PREDICTION_DEGREE).
        let corner_table = self.corner_table.as_ref().unwrap();
        let num_vertices = corner_table.num_vertices();
        let num_faces = corner_table.num_faces();

        let mut point_ids = Vec::with_capacity(num_vertices);
        let mut data_to_corner_map = Vec::with_capacity(num_vertices);

        let mut visited_vertices = vec![false; num_vertices];
        let mut visited_faces = vec![false; num_faces];
        let mut prediction_degree: Vec<i32> = vec![0; num_vertices];

        // Buckets (stacks) for priorities 0..2.
        let mut stacks: [Vec<CornerIndex>; 3] = [Vec::new(), Vec::new(), Vec::new()];
        let mut best_priority: usize = 0;

        let visit_vertex = |v: VertexIndex,
                                c: CornerIndex,
                                point_ids: &mut Vec<PointIndex>,
                                data_to_corner_map: &mut Vec<u32>,
                                visited_vertices: &mut [bool]| {
            if v == INVALID_VERTEX_INDEX {
                return;
            }
            let vi = v.0 as usize;
            if vi >= visited_vertices.len() {
                return;
            }
            if !visited_vertices[vi] {
                visited_vertices[vi] = true;
                point_ids.push(PointIndex(v.0));
                data_to_corner_map.push(c.0);
            }
        };

        let compute_priority = |corner_id: CornerIndex,
                                    visited_vertices: &[bool],
                                    prediction_degree: &mut [i32]| -> usize {
            if corner_id == INVALID_CORNER_INDEX {
                return 2;
            }
            let v_tip = corner_table.vertex(corner_id);
            if v_tip == INVALID_VERTEX_INDEX {
                return 2;
            }
            let vi = v_tip.0 as usize;
            if vi < visited_vertices.len() && visited_vertices[vi] {
                return 0;
            }
            if vi < prediction_degree.len() {
                prediction_degree[vi] += 1;
                if prediction_degree[vi] > 1 {
                    1
                } else {
                    2
                }
            } else {
                2
            }
        };

        let add_corner_to_stack = |ci: CornerIndex,
                                       priority: usize,
                                       stacks: &mut [Vec<CornerIndex>; 3],
                                       best_priority: &mut usize| {
            let p = priority.min(2);
            stacks[p].push(ci);
            if p < *best_priority {
                *best_priority = p;
            }
        };

        let pop_next_corner = |stacks: &mut [Vec<CornerIndex>; 3], best_priority: &mut usize| -> CornerIndex {
            for p in *best_priority..3 {
                if let Some(ci) = stacks[p].pop() {
                    *best_priority = p;
                    return ci;
                }
            }
            INVALID_CORNER_INDEX
        };

        let clear_stacks = |stacks: &mut [Vec<CornerIndex>; 3]| {
            stacks[0].clear();
            stacks[1].clear();
            stacks[2].clear();
        };

        let traverse_from_corner = |start_corner: CornerIndex,
                                        point_ids: &mut Vec<PointIndex>,
                                        data_to_corner_map: &mut Vec<u32>,
                                        visited_vertices: &mut Vec<bool>,
                                        visited_faces: &mut Vec<bool>,
                                        prediction_degree: &mut Vec<i32>,
                                        stacks: &mut [Vec<CornerIndex>; 3],
                                        best_priority: &mut usize| {
            let start_face = corner_table.face(start_corner);
            if start_face == crate::geometry_indices::INVALID_FACE_INDEX {
                return;
            }
            if visited_faces[start_face.0 as usize] {
                return;
            }

            clear_stacks(stacks);
            stacks[0].push(start_corner);
            *best_priority = 0;

            // Pre-visit next, prev and tip vertices.
            let next_c = corner_table.next(start_corner);
            let prev_c = corner_table.previous(start_corner);
            visit_vertex(
                corner_table.vertex(next_c),
                next_c,
                point_ids,
                data_to_corner_map,
                visited_vertices,
            );
            visit_vertex(
                corner_table.vertex(prev_c),
                prev_c,
                point_ids,
                data_to_corner_map,
                visited_vertices,
            );
            visit_vertex(
                corner_table.vertex(start_corner),
                start_corner,
                point_ids,
                data_to_corner_map,
                visited_vertices,
            );

            loop {
                let mut corner_id = pop_next_corner(stacks, best_priority);
                if corner_id == INVALID_CORNER_INDEX {
                    break;
                }
                let face_id0 = corner_table.face(corner_id);
                if face_id0 == crate::geometry_indices::INVALID_FACE_INDEX {
                    continue;
                }
                if visited_faces[face_id0.0 as usize] {
                    continue;
                }

                loop {
                    let face_id = corner_table.face(corner_id);
                    if face_id == crate::geometry_indices::INVALID_FACE_INDEX {
                        break;
                    }
                    visited_faces[face_id.0 as usize] = true;

                    let vert_id = corner_table.vertex(corner_id);
                    if vert_id != INVALID_VERTEX_INDEX {
                        let vi = vert_id.0 as usize;
                        if vi < visited_vertices.len() && !visited_vertices[vi] {
                            visit_vertex(
                                vert_id,
                                corner_id,
                                point_ids,
                                data_to_corner_map,
                                visited_vertices,
                            );
                        }
                    }

                    let right_corner_id = corner_table.right_corner(corner_id);
                    let left_corner_id = corner_table.left_corner(corner_id);
                    let right_face_id = if right_corner_id == INVALID_CORNER_INDEX {
                        crate::geometry_indices::INVALID_FACE_INDEX
                    } else {
                        corner_table.face(right_corner_id)
                    };
                    let left_face_id = if left_corner_id == INVALID_CORNER_INDEX {
                        crate::geometry_indices::INVALID_FACE_INDEX
                    } else {
                        corner_table.face(left_corner_id)
                    };

                    let is_right_face_visited = right_face_id == crate::geometry_indices::INVALID_FACE_INDEX
                        || visited_faces[right_face_id.0 as usize];
                    let is_left_face_visited = left_face_id == crate::geometry_indices::INVALID_FACE_INDEX
                        || visited_faces[left_face_id.0 as usize];

                    if !is_left_face_visited {
                        let priority = compute_priority(left_corner_id, visited_vertices, prediction_degree);
                        if is_right_face_visited && priority <= *best_priority {
                            corner_id = left_corner_id;
                            continue;
                        }
                        add_corner_to_stack(left_corner_id, priority, stacks, best_priority);
                    }

                    if !is_right_face_visited {
                        let priority = compute_priority(right_corner_id, visited_vertices, prediction_degree);
                        if priority <= *best_priority {
                            corner_id = right_corner_id;
                            continue;
                        }
                        add_corner_to_stack(right_corner_id, priority, stacks, best_priority);
                    }

                    break;
                }
            }
        };

        // Traverse all components in face order.
        for f in 0..num_faces {
            if visited_faces[f] {
                continue;
            }
            let first_corner = corner_table.first_corner(FaceIndex(f as u32));
            traverse_from_corner(
                first_corner,
                &mut point_ids,
                &mut data_to_corner_map,
                &mut visited_vertices,
                &mut visited_faces,
                &mut prediction_degree,
                &mut stacks,
                &mut best_priority,
            );
        }

        // Add any remaining isolated vertices.
        for i in 0..num_vertices {
            if !visited_vertices[i] {
                point_ids.push(PointIndex(i as u32));
                let c = corner_table.left_most_corner(VertexIndex(i as u32));
                data_to_corner_map.push(c.0);
            }
        }

        (point_ids, data_to_corner_map)
    }

    fn is_vertex_on_boundary(&self, corner_table: &CornerTable, vert_id: VertexIndex) -> bool {
        let start_c = corner_table.left_most_corner(vert_id);
        if start_c == INVALID_CORNER_INDEX {
            return true;
        }
        let mut c = start_c;
        loop {
            // Edge (c, next(c)) is incident to v.
            if corner_table.opposite(c) == INVALID_CORNER_INDEX {
                return true;
            }
            // Edge (prev(c), c) is also incident to v.
            if corner_table.opposite(corner_table.previous(c)) == INVALID_CORNER_INDEX {
                return true;
            }
            c = corner_table.swing_right(c);
            if c == INVALID_CORNER_INDEX {
                return true;
            }
            if c == start_c {
                break;
            }
        }
        false
    }
}
