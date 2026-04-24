use crate::corner_table::CornerTable;
use crate::decoder_buffer::DecoderBuffer;
use crate::draco_types::DataType;
use crate::geometry_attribute::PointAttribute;
use crate::geometry_indices::{CornerIndex, PointIndex, INVALID_CORNER_INDEX};
use crate::mesh_prediction_scheme_data::MeshPredictionSchemeData;
use crate::point_cloud::PointCloud;
use crate::point_cloud_decoder::PointCloudDecoder;
use crate::prediction_scheme::{
    PredictionScheme, PredictionSchemeDecoder, PredictionSchemeMethod,
    PredictionSchemeTransformType,
};
use crate::prediction_scheme_constrained_multi_parallelogram::PredictionSchemeConstrainedMultiParallelogramDecoder;
use crate::prediction_scheme_delta::PredictionSchemeDeltaDecoder;
use crate::prediction_scheme_geometric_normal::MeshPredictionSchemeGeometricNormalDecoder;
use crate::prediction_scheme_multi_parallelogram::PredictionSchemeMultiParallelogramDecoder;
use crate::prediction_scheme_normal_octahedron_canonicalized_decoding_transform::PredictionSchemeNormalOctahedronCanonicalizedDecodingTransform;
use crate::prediction_scheme_parallelogram::PredictionSchemeParallelogramDecoder;
use crate::prediction_scheme_tex_coords_deprecated::PredictionSchemeTexCoordsDeprecatedDecoder;
use crate::prediction_scheme_tex_coords_portable::PredictionSchemeTexCoordsPortableDecoder;
use crate::prediction_scheme_wrap::PredictionSchemeWrapDecodingTransform;
use crate::symbol_encoding::{decode_symbols, SymbolEncodingOptions};

pub struct SequentialIntegerAttributeDecoder {
    attribute: i32,
    prediction_scheme: Option<Box<dyn PredictionSchemeDecoder<'static, i32, i32>>>,
}

impl Default for SequentialIntegerAttributeDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl SequentialIntegerAttributeDecoder {
    pub fn new() -> Self {
        Self {
            attribute: -1,
            prediction_scheme: None,
        }
    }

    pub fn init(&mut self, _decoder: &PointCloudDecoder, attribute_id: i32) -> bool {
        self.attribute = attribute_id;
        true
    }

    pub fn attribute_id(&self) -> i32 {
        self.attribute
    }

    pub fn set_prediction_scheme(
        &mut self,
        scheme: Box<dyn PredictionSchemeDecoder<'static, i32, i32>>,
    ) {
        self.prediction_scheme = Some(scheme);
    }

    // Complex mesh decoding requires all 8 parameters: mesh data, traversal maps,
    // corner table for prediction, and optional portable attribute output.
    // Refactoring into a struct would obscure the data flow and break C++ API parity.
    #[allow(clippy::too_many_arguments)]
    pub fn decode_values(
        &mut self,
        point_cloud: &mut PointCloud,
        point_ids: &[PointIndex],
        in_buffer: &mut DecoderBuffer,
        corner_table: Option<&CornerTable>,
        data_to_corner_map_override: Option<&[u32]>,
        vertex_to_data_map_override: Option<&[i32]>,
        portable_attribute: Option<&mut PointAttribute>,
        portable_parent_attribute: Option<&PointAttribute>,
        pre_integer_decode: Option<&mut dyn FnMut(&mut DecoderBuffer<'_>) -> bool>,
    ) -> bool {
        let att_id = self.attribute;
        if att_id < 0 {
            return false;
        }

        let num_points = point_ids.len();
        if num_points == 0 {
            return true;
        }

        let attribute = if let Some(ref pa) = portable_attribute {
            &**pa
        } else {
            point_cloud.attribute(att_id)
        };

        let num_components = attribute.num_components() as usize;
        let num_values = num_points * num_components;

        // 3. Decode Prediction Method and (optional) prepare predictor
        let method_byte = match in_buffer.decode_u8() {
            Ok(v) => v,
            Err(_) => {
                eprintln!("Failed to decode prediction method");
                return false;
            }
        };

        // Draco stores prediction method as int8 (0xFF == -1 == None).
        let selected_method = if method_byte == 0xFF {
            PredictionSchemeMethod::None
        } else {
            match PredictionSchemeMethod::try_from(method_byte) {
                Ok(m) => m,
                Err(_) => {
                    return false;
                }
            }
        };

        let mut selected_transform: Option<PredictionSchemeTransformType> = None;
        if selected_method != PredictionSchemeMethod::None {
            // Draco stores prediction transform type as int8 (0xFF == -1 == None).
            let transform_byte = match in_buffer.decode_u8() {
                Ok(v) => v,
                Err(_) => return false,
            };
            if transform_byte != 0xFF {
                match PredictionSchemeTransformType::try_from(transform_byte) {
                    Ok(t) => selected_transform = Some(t),
                    Err(_) => {
                        return false;
                    }
                }
            }
        }

        if let Some(ref scheme) = self.prediction_scheme {
            // println!("DEBUG: Decoder scheme method: {:?}", scheme.get_prediction_method());
            if scheme.get_prediction_method() != selected_method {
                eprintln!(
                    "Prediction method mismatch. Stream: {:?}, Scheme: {:?}",
                    selected_method,
                    scheme.get_prediction_method()
                );
                return false;
            }
        }

        let mut predictor_opt: Option<
            PredictionSchemeDeltaDecoder<i32, i32, PredictionSchemeWrapDecodingTransform<i32>>,
        > = None;
        let mut predictor_normal_octa_diff_opt: Option<
            PredictionSchemeDeltaDecoder<
                i32,
                i32,
                PredictionSchemeNormalOctahedronCanonicalizedDecodingTransform,
            >,
        > = None;
        let mut predictor_parallelogram_opt: Option<
            PredictionSchemeParallelogramDecoder<
                i32,
                i32,
                PredictionSchemeWrapDecodingTransform<i32>,
            >,
        > = None;
        let mut predictor_multi_parallelogram_opt: Option<
            PredictionSchemeMultiParallelogramDecoder<
                '_,
                i32,
                i32,
                PredictionSchemeWrapDecodingTransform<i32>,
            >,
        > = None;
        let mut predictor_constrained_multi_parallelogram_opt: Option<
            PredictionSchemeConstrainedMultiParallelogramDecoder<
                '_,
                i32,
                i32,
                PredictionSchemeWrapDecodingTransform<i32>,
            >,
        > = None;
        let mut predictor_tex_coords_deprecated_opt: Option<
            PredictionSchemeTexCoordsDeprecatedDecoder<
                '_,
                PredictionSchemeWrapDecodingTransform<i32>,
            >,
        > = None;
        let mut predictor_tex_coords_opt: Option<PredictionSchemeTexCoordsPortableDecoder> = None;
        let mut predictor_geometric_normal_opt: Option<MeshPredictionSchemeGeometricNormalDecoder> =
            None;

        // Maps need to live long enough
        let mut vertex_to_data_map: Vec<i32> = Vec::new();
        let mut data_to_corner_map: Vec<u32> = Vec::new();
        match selected_method {
            _ if self.prediction_scheme.is_some() => {
                // Do nothing, scheme already set
            }
            PredictionSchemeMethod::Difference => match selected_transform {
                Some(PredictionSchemeTransformType::NormalOctahedronCanonicalized) => {
                    let transform =
                        PredictionSchemeNormalOctahedronCanonicalizedDecodingTransform::new();
                    let predictor = PredictionSchemeDeltaDecoder::new(transform);
                    predictor_normal_octa_diff_opt = Some(predictor);
                }
                _ => {
                    let transform = PredictionSchemeWrapDecodingTransform::<i32>::new();
                    let predictor = PredictionSchemeDeltaDecoder::new(transform);
                    predictor_opt = Some(predictor);
                }
            },
            PredictionSchemeMethod::MeshPredictionParallelogram => {
                if let Some(corner_table) = corner_table {
                    // Generate maps
                    data_to_corner_map.resize(num_points, 0);

                    // vertex_to_data_map_override takes priority when available
                    // (it's built by the decoder's own DFS traversal)
                    if let Some(map) = vertex_to_data_map_override {
                        // Use the pre-built vertex_to_data_map from mesh decoder
                        if map.len() != corner_table.num_vertices() {
                            eprintln!("Invalid vertex_to_data_map_override length");
                            return false;
                        }
                        vertex_to_data_map.resize(map.len(), 0);
                        vertex_to_data_map.copy_from_slice(map);

                        // Also set data_to_corner_map if override is available
                        if let Some(dcm) = data_to_corner_map_override {
                            if dcm.len() != num_points {
                                eprintln!("Invalid data_to_corner_map_override length");
                                return false;
                            }
                            data_to_corner_map.copy_from_slice(dcm);
                        }
                    } else if let Some(map) = data_to_corner_map_override {
                        if map.len() != num_points {
                            eprintln!("Invalid data_to_corner_map_override length");
                            return false;
                        }
                        data_to_corner_map.copy_from_slice(map);

                        // When using an override, the corner table may contain seam-split
                        // vertices with ids outside the original point range. Build the
                        // vertex->data map from the data->corner map.
                        vertex_to_data_map.resize(corner_table.num_vertices(), -1);
                        for (data_id, &corner_u32) in data_to_corner_map.iter().enumerate() {
                            let corner_id = CornerIndex(corner_u32);
                            if corner_id == INVALID_CORNER_INDEX {
                                continue;
                            }
                            let v = corner_table.vertex(corner_id).0 as usize;
                            if v < vertex_to_data_map.len() {
                                vertex_to_data_map[v] = data_id as i32;
                            }
                        }
                    } else {
                        // Build vertex_to_data_map from data_to_corner_map using corner table vertex IDs
                        // This is the same logic as the 'if' branch above
                        vertex_to_data_map.resize(corner_table.num_vertices(), -1);
                        for (data_id, &corner_u32) in data_to_corner_map.iter().enumerate() {
                            let corner_id = CornerIndex(corner_u32);
                            if corner_id == INVALID_CORNER_INDEX {
                                continue;
                            }
                            let v = corner_table.vertex(corner_id).0 as usize;
                            if v < vertex_to_data_map.len() {
                                vertex_to_data_map[v] = data_id as i32;
                            }
                        }
                    }

                    let mut mesh_data = MeshPredictionSchemeData::new();
                    mesh_data.set(corner_table, &data_to_corner_map, &vertex_to_data_map);

                    let transform = PredictionSchemeWrapDecodingTransform::<i32>::new();
                    let predictor =
                        PredictionSchemeParallelogramDecoder::new(attribute, transform, mesh_data);
                    predictor_parallelogram_opt = Some(predictor);
                } else {
                    eprintln!("Parallelogram prediction requires corner table");
                    return false;
                }
            }
            PredictionSchemeMethod::MeshPredictionMultiParallelogram => {
                if let Some(corner_table) = corner_table {
                    data_to_corner_map.resize(num_points, 0);

                    if let Some(map) = vertex_to_data_map_override {
                        if map.len() != corner_table.num_vertices() {
                            eprintln!("Invalid vertex_to_data_map_override length");
                            return false;
                        }
                        vertex_to_data_map.resize(map.len(), 0);
                        vertex_to_data_map.copy_from_slice(map);

                        if let Some(dcm) = data_to_corner_map_override {
                            if dcm.len() != num_points {
                                eprintln!("Invalid data_to_corner_map_override length");
                                return false;
                            }
                            data_to_corner_map.copy_from_slice(dcm);
                        }
                    } else if let Some(map) = data_to_corner_map_override {
                        if map.len() != num_points {
                            eprintln!("Invalid data_to_corner_map_override length");
                            return false;
                        }
                        data_to_corner_map.copy_from_slice(map);

                        vertex_to_data_map.resize(corner_table.num_vertices(), -1);
                        for (data_id, &corner_u32) in data_to_corner_map.iter().enumerate() {
                            let corner_id = CornerIndex(corner_u32);
                            if corner_id == INVALID_CORNER_INDEX {
                                continue;
                            }
                            let v = corner_table.vertex(corner_id).0 as usize;
                            if v < vertex_to_data_map.len() {
                                vertex_to_data_map[v] = data_id as i32;
                            }
                        }
                    } else {
                        vertex_to_data_map.resize(corner_table.num_vertices(), -1);
                        for (data_id, &corner_u32) in data_to_corner_map.iter().enumerate() {
                            let corner_id = CornerIndex(corner_u32);
                            if corner_id == INVALID_CORNER_INDEX {
                                continue;
                            }
                            let v = corner_table.vertex(corner_id).0 as usize;
                            if v < vertex_to_data_map.len() {
                                vertex_to_data_map[v] = data_id as i32;
                            }
                        }
                    }

                    let mut mesh_data = MeshPredictionSchemeData::new();
                    mesh_data.set(corner_table, &data_to_corner_map, &vertex_to_data_map);

                    let transform = PredictionSchemeWrapDecodingTransform::<i32>::new();
                    let predictor =
                        PredictionSchemeMultiParallelogramDecoder::new(transform, mesh_data);
                    predictor_multi_parallelogram_opt = Some(predictor);
                } else {
                    eprintln!("MultiParallelogram prediction requires corner table");
                    return false;
                }
            }
            PredictionSchemeMethod::MeshPredictionConstrainedMultiParallelogram => {
                if let Some(corner_table) = corner_table {
                    // Generate maps
                    data_to_corner_map.resize(num_points, 0);

                    // vertex_to_data_map_override takes priority when available
                    // (it's built by the decoder's own DFS traversal)
                    if let Some(map) = vertex_to_data_map_override {
                        // Use the pre-built vertex_to_data_map from mesh decoder
                        if map.len() != corner_table.num_vertices() {
                            eprintln!("Invalid vertex_to_data_map_override length");
                            return false;
                        }
                        vertex_to_data_map.resize(map.len(), 0);
                        vertex_to_data_map.copy_from_slice(map);

                        // Also set data_to_corner_map if override is available
                        if let Some(dcm) = data_to_corner_map_override {
                            if dcm.len() != num_points {
                                eprintln!("Invalid data_to_corner_map_override length");
                                return false;
                            }
                            data_to_corner_map.copy_from_slice(dcm);
                        }
                    } else if let Some(map) = data_to_corner_map_override {
                        if map.len() != num_points {
                            eprintln!("Invalid data_to_corner_map_override length");
                            return false;
                        }
                        data_to_corner_map.copy_from_slice(map);

                        vertex_to_data_map.resize(corner_table.num_vertices(), -1);
                        for (data_id, &corner_u32) in data_to_corner_map.iter().enumerate() {
                            let corner_id = CornerIndex(corner_u32);
                            if corner_id == INVALID_CORNER_INDEX {
                                continue;
                            }
                            let v = corner_table.vertex(corner_id).0 as usize;
                            if v < vertex_to_data_map.len() {
                                vertex_to_data_map[v] = data_id as i32;
                            }
                        }
                    } else {
                        // Build vertex_to_data_map from data_to_corner_map using corner table vertex IDs
                        // This is the same logic as the 'if' branch above
                        vertex_to_data_map.resize(corner_table.num_vertices(), -1);
                        for (data_id, &corner_u32) in data_to_corner_map.iter().enumerate() {
                            let corner_id = CornerIndex(corner_u32);
                            if corner_id == INVALID_CORNER_INDEX {
                                continue;
                            }
                            let v = corner_table.vertex(corner_id).0 as usize;
                            if v < vertex_to_data_map.len() {
                                vertex_to_data_map[v] = data_id as i32;
                            }
                        }
                    }

                    let mut mesh_data = MeshPredictionSchemeData::new();
                    mesh_data.set(corner_table, &data_to_corner_map, &vertex_to_data_map);

                    let transform = PredictionSchemeWrapDecodingTransform::<i32>::new();
                    let predictor = PredictionSchemeConstrainedMultiParallelogramDecoder::new(
                        transform, mesh_data,
                    );
                    predictor_constrained_multi_parallelogram_opt = Some(predictor);
                } else {
                    eprintln!("ConstrainedMultiParallelogram prediction requires corner table");
                    return false;
                }
            }
            PredictionSchemeMethod::MeshPredictionTexCoordsDeprecated => {
                if let Some(corner_table) = corner_table {
                    data_to_corner_map.resize(num_points, 0);

                    if let Some(map) = vertex_to_data_map_override {
                        if map.len() != corner_table.num_vertices() {
                            eprintln!("Invalid vertex_to_data_map_override length");
                            return false;
                        }
                        vertex_to_data_map.resize(map.len(), 0);
                        vertex_to_data_map.copy_from_slice(map);

                        if let Some(dcm) = data_to_corner_map_override {
                            if dcm.len() != num_points {
                                eprintln!("Invalid data_to_corner_map_override length");
                                return false;
                            }
                            data_to_corner_map.copy_from_slice(dcm);
                        }
                    } else if let Some(map) = data_to_corner_map_override {
                        if map.len() != num_points {
                            eprintln!("Invalid data_to_corner_map_override length");
                            return false;
                        }
                        data_to_corner_map.copy_from_slice(map);

                        vertex_to_data_map.resize(corner_table.num_vertices(), -1);
                        for (data_id, &corner_u32) in data_to_corner_map.iter().enumerate() {
                            let corner_id = CornerIndex(corner_u32);
                            if corner_id == INVALID_CORNER_INDEX {
                                continue;
                            }
                            let v = corner_table.vertex(corner_id).0 as usize;
                            if v < vertex_to_data_map.len() {
                                vertex_to_data_map[v] = data_id as i32;
                            }
                        }
                    } else {
                        vertex_to_data_map.resize(corner_table.num_vertices(), -1);
                        for (data_id, &corner_u32) in data_to_corner_map.iter().enumerate() {
                            let corner_id = CornerIndex(corner_u32);
                            if corner_id == INVALID_CORNER_INDEX {
                                continue;
                            }
                            let v = corner_table.vertex(corner_id).0 as usize;
                            if v < vertex_to_data_map.len() {
                                vertex_to_data_map[v] = data_id as i32;
                            }
                        }
                    }

                    let mut mesh_data = MeshPredictionSchemeData::new();
                    mesh_data.set(corner_table, &data_to_corner_map, &vertex_to_data_map);

                    let transform = PredictionSchemeWrapDecodingTransform::<i32>::new();
                    let mut predictor = PredictionSchemeTexCoordsDeprecatedDecoder::new(transform);
                    predictor.init(&mesh_data);

                    let pos_att_id = point_cloud.named_attribute_id(
                        crate::geometry_attribute::GeometryAttributeType::Position,
                    );
                    if pos_att_id >= 0 {
                        let pos_att = portable_parent_attribute
                            .unwrap_or_else(|| point_cloud.attribute(pos_att_id));
                        if !predictor.set_parent_attribute(pos_att) {
                            eprintln!("Failed to set parent attribute for TexCoordsDeprecated");
                            return false;
                        }
                    } else {
                        eprintln!("Position attribute not found for TexCoordsDeprecated");
                        return false;
                    }

                    predictor_tex_coords_deprecated_opt = Some(predictor);
                } else {
                    eprintln!("TexCoordsDeprecated prediction requires corner table");
                    return false;
                }
            }
            PredictionSchemeMethod::MeshPredictionTexCoordsPortable => {
                if let Some(corner_table) = corner_table {
                    data_to_corner_map.resize(num_points, 0);

                    // vertex_to_data_map_override takes priority when available
                    // (it's built by the decoder's own DFS traversal)
                    if let Some(map) = vertex_to_data_map_override {
                        // Use the pre-built vertex_to_data_map from mesh decoder
                        if map.len() != corner_table.num_vertices() {
                            eprintln!("Invalid vertex_to_data_map_override length");
                            return false;
                        }
                        vertex_to_data_map.resize(map.len(), 0);
                        vertex_to_data_map.copy_from_slice(map);

                        // Also set data_to_corner_map if override is available
                        if let Some(dcm) = data_to_corner_map_override {
                            if dcm.len() != num_points {
                                eprintln!("Invalid data_to_corner_map_override length");
                                return false;
                            }
                            data_to_corner_map.copy_from_slice(dcm);
                        }
                    } else if let Some(map) = data_to_corner_map_override {
                        if map.len() != num_points {
                            eprintln!("Invalid data_to_corner_map_override length");
                            return false;
                        }
                        data_to_corner_map.copy_from_slice(map);

                        vertex_to_data_map.resize(corner_table.num_vertices(), -1);
                        for (data_id, &corner_u32) in data_to_corner_map.iter().enumerate() {
                            let corner_id = CornerIndex(corner_u32);
                            if corner_id == INVALID_CORNER_INDEX {
                                continue;
                            }
                            let v = corner_table.vertex(corner_id).0 as usize;
                            if v < vertex_to_data_map.len() {
                                vertex_to_data_map[v] = data_id as i32;
                            }
                        }
                    } else {
                        // Build vertex_to_data_map from data_to_corner_map using corner table vertex IDs
                        // This is the same logic as the 'if' branch above
                        vertex_to_data_map.resize(corner_table.num_vertices(), -1);
                        for (data_id, &corner_u32) in data_to_corner_map.iter().enumerate() {
                            let corner_id = CornerIndex(corner_u32);
                            if corner_id == INVALID_CORNER_INDEX {
                                continue;
                            }
                            let v = corner_table.vertex(corner_id).0 as usize;
                            if v < vertex_to_data_map.len() {
                                vertex_to_data_map[v] = data_id as i32;
                            }
                        }
                    }

                    let mut mesh_data = MeshPredictionSchemeData::new();
                    mesh_data.set(corner_table, &data_to_corner_map, &vertex_to_data_map);

                    let transform = PredictionSchemeWrapDecodingTransform::<i32>::new();
                    let mut predictor = PredictionSchemeTexCoordsPortableDecoder::new(transform);
                    predictor.init(&mesh_data);

                    // Set parent attribute (Position)
                    let pos_att_id = point_cloud.named_attribute_id(
                        crate::geometry_attribute::GeometryAttributeType::Position,
                    );
                    if pos_att_id >= 0 {
                        let pos_att = portable_parent_attribute
                            .unwrap_or_else(|| point_cloud.attribute(pos_att_id));
                        if !predictor.set_parent_attribute(pos_att) {
                            eprintln!("Failed to set parent attribute for TexCoordsPortable");
                            return false;
                        }
                    } else {
                        eprintln!("Position attribute not found for TexCoordsPortable");
                        return false;
                    }

                    predictor_tex_coords_opt = Some(predictor);
                } else {
                    eprintln!("TexCoordsPortable prediction requires corner table");
                    return false;
                }
            }
            PredictionSchemeMethod::MeshPredictionGeometricNormal => {
                if let Some(corner_table) = corner_table {
                    data_to_corner_map.resize(num_points, 0);

                    // vertex_to_data_map_override takes priority when available
                    // (it's built by the decoder's own DFS traversal)
                    if let Some(map) = vertex_to_data_map_override {
                        // Use the pre-built vertex_to_data_map from mesh decoder
                        if map.len() != corner_table.num_vertices() {
                            eprintln!("Invalid vertex_to_data_map_override length");
                            return false;
                        }
                        vertex_to_data_map.resize(map.len(), 0);
                        vertex_to_data_map.copy_from_slice(map);

                        // Also set data_to_corner_map if override is available
                        if let Some(dcm) = data_to_corner_map_override {
                            if dcm.len() != num_points {
                                eprintln!("Invalid data_to_corner_map_override length");
                                return false;
                            }
                            data_to_corner_map.copy_from_slice(dcm);
                        }
                    } else if let Some(map) = data_to_corner_map_override {
                        if map.len() != num_points {
                            eprintln!("Invalid data_to_corner_map_override length");
                            return false;
                        }
                        data_to_corner_map.copy_from_slice(map);

                        vertex_to_data_map.resize(corner_table.num_vertices(), -1);
                        for (data_id, &corner_u32) in data_to_corner_map.iter().enumerate() {
                            let corner_id = CornerIndex(corner_u32);
                            if corner_id == INVALID_CORNER_INDEX {
                                continue;
                            }
                            let v = corner_table.vertex(corner_id).0 as usize;
                            if v < vertex_to_data_map.len() {
                                vertex_to_data_map[v] = data_id as i32;
                            }
                        }
                    } else {
                        // Build vertex_to_data_map from data_to_corner_map using corner table vertex IDs
                        // This is the same logic as the 'if' branch above
                        vertex_to_data_map.resize(corner_table.num_vertices(), -1);
                        for (data_id, &corner_u32) in data_to_corner_map.iter().enumerate() {
                            let corner_id = CornerIndex(corner_u32);
                            if corner_id == INVALID_CORNER_INDEX {
                                continue;
                            }
                            let v = corner_table.vertex(corner_id).0 as usize;
                            if v < vertex_to_data_map.len() {
                                vertex_to_data_map[v] = data_id as i32;
                            }
                        }
                    }

                    let mut mesh_data = MeshPredictionSchemeData::new();
                    mesh_data.set(corner_table, &data_to_corner_map, &vertex_to_data_map);

                    let transform =
                        PredictionSchemeNormalOctahedronCanonicalizedDecodingTransform::new();
                    let mut predictor = MeshPredictionSchemeGeometricNormalDecoder::new(transform);
                    predictor.init(&mesh_data);

                    // Provide mapping from decoded-entry index to original point id.
                    predictor.set_entry_to_point_id_map(point_ids);

                    // Set parent attribute (Position)
                    let pos_att_id = point_cloud.named_attribute_id(
                        crate::geometry_attribute::GeometryAttributeType::Position,
                    );
                    if pos_att_id >= 0 {
                        let pos_att = portable_parent_attribute
                            .unwrap_or_else(|| point_cloud.attribute(pos_att_id));
                        if !predictor.set_parent_attribute(pos_att) {
                            eprintln!("Failed to set parent attribute for GeometricNormal");
                            return false;
                        }
                    } else {
                        eprintln!("Position attribute not found for GeometricNormal");
                        return false;
                    }

                    predictor_geometric_normal_opt = Some(predictor);
                } else {
                    eprintln!("GeometricNormal prediction requires corner table");
                    return false;
                }
            }
            PredictionSchemeMethod::None => {}
            _ => {
                eprintln!("Unsupported prediction method: {:?}", selected_method);
                return false;
            }
        }

        // 1. Decode correction symbols.
        // For v < 2.0, transform-specific parameters (quantization, octahedron)
        // are stored BEFORE the integer values. The caller provides a hook.
        if let Some(hook) = pre_integer_decode {
            if !hook(in_buffer) {
                return false;
            }
        }
        // Draco supports both entropy-coded symbols (compressed=1) and raw symbols (compressed=0).
        let compressed = match in_buffer.decode_u8() {
            Ok(v) => v,
            Err(_) => return false,
        };

        let mut symbols = vec![0u32; num_values];

        // Check if the prediction scheme produces positive corrections (no ZigZag needed)
        // Octahedron transforms (for normals) produce positive corrections
        let are_corrections_positive = match selected_transform {
            Some(PredictionSchemeTransformType::NormalOctahedron)
            | Some(PredictionSchemeTransformType::NormalOctahedronCanonicalized) => true,
            _ => {
                // Fallback: check self.prediction_scheme if it's set
                if let Some(ref scheme) = self.prediction_scheme {
                    scheme.are_corrections_positive()
                } else {
                    false
                }
            }
        };

        let needs_zigzag_conversion;
        if compressed > 0 {
            // Entropy-coded symbols are zigzag encoded UNLESS the prediction scheme
            // guarantees positive corrections (e.g., normal octahedron transform)
            needs_zigzag_conversion = !are_corrections_positive;
            let options = SymbolEncodingOptions::default();
            if !decode_symbols(
                num_values,
                num_components,
                &options,
                in_buffer,
                &mut symbols,
            ) {
                return false;
            }
        } else {
            // Raw uncompressed integers. Read directly as bytes.
            // ZigZag conversion is needed unless the scheme guarantees positive corrections.
            needs_zigzag_conversion = !are_corrections_positive;

            let num_bytes = match in_buffer.decode_u8() {
                Ok(v) => v as usize,
                Err(_) => return false,
            };
            if num_bytes > 4 {
                return false;
            }

            if num_bytes == 0 {
                // All values are zero — nothing to read from the buffer.
            } else if num_bytes == 4 {
                for s in &mut symbols {
                    *s = match in_buffer.decode_u32() {
                        Ok(v) => v,
                        Err(_) => return false,
                    };
                }
            } else {
                for s in &mut symbols {
                    let mut tmp = [0u8; 4];
                    if in_buffer.decode_bytes(&mut tmp[..num_bytes]).is_err() {
                        return false;
                    }
                    *s = u32::from_le_bytes(tmp);
                }
            }
        }

        // 2. Convert symbols to signed corrections in-place.
        let corrections: Vec<i32> = if needs_zigzag_conversion {
            symbols
                .into_iter()
                .map(|s| ((s >> 1) as i32) ^ (-((s & 1) as i32)))
                .collect()
        } else {
            symbols.into_iter().map(|s| s as i32).collect()
        };

        // Initialize values array that will be computed by prediction schemes
        let mut values = vec![0i32; num_values];

        // 3. Decode prediction scheme data (if any).
        match selected_method {
            _ if self.prediction_scheme.is_some() => {
                let scheme = self.prediction_scheme.as_mut().unwrap();
                if !scheme.decode_prediction_data(in_buffer) {
                    eprintln!(
                        "Failed to decode prediction data (att_id={}, method={:?}, transform={:?})",
                        att_id, selected_method, selected_transform
                    );
                    return false;
                }
            }
            PredictionSchemeMethod::Difference => {
                if let Some(predictor) = predictor_normal_octa_diff_opt.as_mut() {
                    if !predictor.decode_prediction_data(in_buffer) {
                        eprintln!(
                            "Failed to decode prediction data (att_id={}, method={:?}, transform={:?})",
                            att_id, selected_method, selected_transform
                        );
                        return false;
                    }
                } else {
                    let predictor = predictor_opt.as_mut().unwrap();
                    if !predictor.decode_prediction_data(in_buffer) {
                        eprintln!(
                            "Failed to decode prediction data (att_id={}, method={:?}, transform={:?})",
                            att_id, selected_method, selected_transform
                        );
                        return false;
                    }
                }
            }
            PredictionSchemeMethod::MeshPredictionParallelogram => {
                let predictor = predictor_parallelogram_opt.as_mut().unwrap();
                if !predictor.decode_prediction_data(in_buffer) {
                    eprintln!(
                        "Failed to decode prediction data (att_id={}, method={:?}, transform={:?})",
                        att_id, selected_method, selected_transform
                    );
                    return false;
                }
            }
            PredictionSchemeMethod::MeshPredictionMultiParallelogram => {
                let predictor = predictor_multi_parallelogram_opt.as_mut().unwrap();
                if !predictor.decode_prediction_data(in_buffer) {
                    eprintln!(
                        "Failed to decode prediction data (att_id={}, method={:?}, transform={:?})",
                        att_id, selected_method, selected_transform
                    );
                    return false;
                }
            }
            PredictionSchemeMethod::MeshPredictionConstrainedMultiParallelogram => {
                let predictor = predictor_constrained_multi_parallelogram_opt
                    .as_mut()
                    .unwrap();
                if !predictor.decode_prediction_data(in_buffer) {
                    eprintln!(
                        "Failed to decode prediction data (att_id={}, method={:?}, transform={:?})",
                        att_id, selected_method, selected_transform
                    );
                    return false;
                }
            }
            PredictionSchemeMethod::MeshPredictionTexCoordsDeprecated => {
                let predictor = predictor_tex_coords_deprecated_opt.as_mut().unwrap();
                if !predictor.decode_prediction_data(in_buffer) {
                    eprintln!(
                        "Failed to decode prediction data (att_id={}, method={:?}, transform={:?})",
                        att_id, selected_method, selected_transform
                    );
                    return false;
                }
            }
            PredictionSchemeMethod::MeshPredictionTexCoordsPortable => {
                let predictor = predictor_tex_coords_opt.as_mut().unwrap();
                if !predictor.decode_prediction_data(in_buffer) {
                    eprintln!(
                        "Failed to decode prediction data (att_id={}, method={:?}, transform={:?})",
                        att_id, selected_method, selected_transform
                    );
                    return false;
                }
            }
            PredictionSchemeMethod::MeshPredictionGeometricNormal => {
                let predictor = predictor_geometric_normal_opt.as_mut().unwrap();
                if !predictor.decode_prediction_data(in_buffer) {
                    eprintln!(
                        "Failed to decode prediction data (att_id={}, method={:?}, transform={:?})",
                        att_id, selected_method, selected_transform
                    );
                    return false;
                }
            }
            PredictionSchemeMethod::None => {}
            _ => {
                return false;
            }
        }

        // 4. Apply Inverse Prediction.
        match selected_method {
            _ if self.prediction_scheme.is_some() => {
                let scheme = self.prediction_scheme.as_mut().unwrap();
                let entry_to_point_id_map: Vec<u32> = point_ids.iter().map(|p| p.0).collect();
                let map_opt = match selected_method {
                    PredictionSchemeMethod::MeshPredictionParallelogram
                    | PredictionSchemeMethod::MeshPredictionMultiParallelogram
                    | PredictionSchemeMethod::MeshPredictionConstrainedMultiParallelogram
                    | PredictionSchemeMethod::MeshPredictionTexCoordsDeprecated
                    | PredictionSchemeMethod::MeshPredictionTexCoordsPortable
                    | PredictionSchemeMethod::MeshPredictionGeometricNormal => {
                        Some(entry_to_point_id_map.as_slice())
                    }
                    _ => None,
                };
                if !scheme.compute_original_values(
                    &corrections,
                    &mut values,
                    num_values,
                    num_components,
                    map_opt,
                ) {
                    eprintln!(
                        "Failed to compute original values (att_id={}, method={:?}, transform={:?})",
                        att_id, selected_method, selected_transform
                    );
                    return false;
                }
            }
            PredictionSchemeMethod::Difference => {
                if let Some(predictor) = predictor_normal_octa_diff_opt.as_mut() {
                    if !predictor.compute_original_values(
                        &corrections,
                        &mut values,
                        num_values,
                        num_components,
                        None,
                    ) {
                        eprintln!(
                            "Failed to compute original values (att_id={}, method={:?}, transform={:?})",
                            att_id, selected_method, selected_transform
                        );
                        return false;
                    }
                } else {
                    let predictor = predictor_opt.as_mut().unwrap();
                    if !predictor.compute_original_values(
                        &corrections,
                        &mut values,
                        num_values,
                        num_components,
                        None,
                    ) {
                        eprintln!(
                            "Failed to compute original values (att_id={}, method={:?}, transform={:?})",
                            att_id, selected_method, selected_transform
                        );
                        return false;
                    }
                }
            }
            PredictionSchemeMethod::MeshPredictionParallelogram => {
                let predictor = predictor_parallelogram_opt.as_mut().unwrap();
                if !predictor.compute_original_values(
                    &corrections,
                    &mut values,
                    num_values,
                    num_components,
                    None,
                ) {
                    eprintln!(
                        "Failed to compute original values (att_id={}, method={:?}, transform={:?})",
                        att_id, selected_method, selected_transform
                    );
                    return false;
                }
            }
            PredictionSchemeMethod::MeshPredictionMultiParallelogram => {
                let predictor = predictor_multi_parallelogram_opt.as_mut().unwrap();
                if !predictor.compute_original_values(
                    &corrections,
                    &mut values,
                    num_values,
                    num_components,
                    None,
                ) {
                    eprintln!(
                        "Failed to compute original values (att_id={}, method={:?}, transform={:?})",
                        att_id, selected_method, selected_transform
                    );
                    return false;
                }
            }
            PredictionSchemeMethod::MeshPredictionConstrainedMultiParallelogram => {
                let predictor = predictor_constrained_multi_parallelogram_opt
                    .as_mut()
                    .unwrap();
                if !predictor.compute_original_values(
                    &corrections,
                    &mut values,
                    num_values,
                    num_components,
                    None,
                ) {
                    eprintln!(
                        "Failed to compute original values (att_id={}, method={:?}, transform={:?})",
                        att_id, selected_method, selected_transform
                    );
                    return false;
                }
            }
            PredictionSchemeMethod::MeshPredictionTexCoordsDeprecated => {
                let predictor = predictor_tex_coords_deprecated_opt.as_mut().unwrap();
                let entry_to_point_id_map: Vec<u32> = point_ids.iter().map(|p| p.0).collect();
                if !predictor.compute_original_values(
                    &corrections,
                    &mut values,
                    num_values,
                    num_components,
                    Some(&entry_to_point_id_map),
                ) {
                    eprintln!(
                        "Failed to compute original values (att_id={}, method={:?}, transform={:?})",
                        att_id, selected_method, selected_transform
                    );
                    return false;
                }
            }
            PredictionSchemeMethod::MeshPredictionTexCoordsPortable => {
                let predictor = predictor_tex_coords_opt.as_mut().unwrap();
                let entry_to_point_id_map: Vec<u32> = point_ids.iter().map(|p| p.0).collect();
                if !predictor.compute_original_values(
                    &corrections,
                    &mut values,
                    num_values,
                    num_components,
                    Some(&entry_to_point_id_map),
                ) {
                    eprintln!(
                        "Failed to compute original values (att_id={}, method={:?}, transform={:?})",
                        att_id, selected_method, selected_transform
                    );
                    return false;
                }
            }
            PredictionSchemeMethod::MeshPredictionGeometricNormal => {
                let predictor = predictor_geometric_normal_opt.as_mut().unwrap();
                let entry_to_point_id_map: Vec<u32> = point_ids.iter().map(|p| p.0).collect();
                if !predictor.compute_original_values(
                    &corrections,
                    &mut values,
                    num_values,
                    num_components,
                    Some(&entry_to_point_id_map),
                ) {
                    eprintln!(
                        "Failed to compute original values (att_id={}, method={:?}, transform={:?})",
                        att_id, selected_method, selected_transform
                    );
                    return false;
                }
            }
            PredictionSchemeMethod::None => {
                values.copy_from_slice(&corrections);
            }
            _ => {
                eprintln!("Unsupported prediction method: {:?}", selected_method);
                return false;
            }
        }

        if num_points > 0 && cfg!(feature = "debug_logs") {
            println!(
                "Sequential Decoded: Point 0 ID = {:?}, Value[0] = {}",
                point_ids[0], values[0]
            );
            // Debug: print all decoded values (quantized) and where they go
            println!("DEBUG decoded values (first 25 x/y/z):");
            for i in 0..std::cmp::min(25, num_points) {
                let x = values[i * 3];
                let y = values[i * 3 + 1];
                let z = values[i * 3 + 2];
                println!(
                    "  data_id={} -> point_ids[{}]={:?}: quantized({}, {}, {})",
                    i, i, point_ids[i], x, y, z
                );
            }
        }

        // 5. Store values (+ optional inverse transform)
        if let Some(portable_att) = portable_attribute {
            store_i32_values_to_attribute(portable_att, &values, num_points, num_components);
        } else {
            let dst_attribute = point_cloud.attribute_mut(att_id);
            store_i32_values_to_attribute(dst_attribute, &values, num_points, num_components);
        }

        true
    }
}

/// Store decoded i32 values into an attribute buffer.
/// Uses bulk memcpy when the attribute layout matches i32/u32 tightly packed.
#[inline]
fn store_i32_values_to_attribute(
    attr: &mut PointAttribute,
    values: &[i32],
    num_points: usize,
    num_components: usize,
) {
    let byte_stride = attr.byte_stride() as usize;
    let data_type = attr.data_type();
    let component_size = data_type.byte_length();
    let packed_row = num_components * component_size;

    // Ensure buffer is large enough for num_points entries.
    let required = num_points * byte_stride;
    if attr.buffer().data_size() < required {
        attr.buffer_mut().resize(required);
    }

    // Fast path: i32/u32 tightly packed — bulk memcpy the entire values array.
    if (data_type == DataType::Int32 || data_type == DataType::Uint32) && byte_stride == packed_row
    {
        let n = std::cmp::min(num_points, values.len() / num_components);
        let src: &[u8] = bytemuck::cast_slice(&values[..n * num_components]);
        let dst = attr.buffer_mut().data_mut();
        let copy_len = std::cmp::min(src.len(), dst.len());
        dst[..copy_len].copy_from_slice(&src[..copy_len]);
        return;
    }

    // Slow path: per-component write with type conversion.
    let dst_buffer = attr.buffer_mut();
    let num_values_decoded = values.len() / num_components;
    for i in 0..std::cmp::min(num_points, num_values_decoded) {
        let entry_offset = i * byte_stride;
        for c in 0..num_components {
            let component_offset = entry_offset + c * component_size;
            write_value_from_i32(
                dst_buffer,
                component_offset,
                data_type,
                values[i * num_components + c],
            );
        }
    }
}

#[inline(always)]
fn write_value_from_i32(
    buffer: &mut crate::data_buffer::DataBuffer,
    offset: usize,
    data_type: DataType,
    val: i32,
) {
    match data_type {
        DataType::Int8 => {
            buffer.write(offset, &(val as i8).to_le_bytes());
        }
        DataType::Uint8 => {
            buffer.write(offset, &(val as u8).to_le_bytes());
        }
        DataType::Int16 => {
            buffer.write(offset, &(val as i16).to_le_bytes());
        }
        DataType::Uint16 => {
            buffer.write(offset, &(val as u16).to_le_bytes());
        }
        DataType::Int32 => {
            buffer.write(offset, &val.to_le_bytes());
        }
        DataType::Uint32 => {
            buffer.write(offset, &(val as u32).to_le_bytes());
        }
        _ => {}
    }
}
