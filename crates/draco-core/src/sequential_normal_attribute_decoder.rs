use crate::sequential_integer_attribute_decoder::SequentialIntegerAttributeDecoder;
use crate::attribute_octahedron_transform::AttributeOctahedronTransform;
use crate::decoder_buffer::DecoderBuffer;
use crate::point_cloud::PointCloud;
use crate::geometry_indices::PointIndex;
use crate::point_cloud_decoder::PointCloudDecoder;
use crate::prediction_scheme_normal_octahedron_canonicalized_decoding_transform::PredictionSchemeNormalOctahedronCanonicalizedDecodingTransform;
use crate::status::{Status, DracoError};
use crate::draco_types::DataType;
use crate::attribute_transform::AttributeTransform;
use crate::corner_table::CornerTable;

use crate::prediction_scheme_delta::PredictionSchemeDeltaDecoder;

pub struct SequentialNormalAttributeDecoder {
    base: SequentialIntegerAttributeDecoder,
    attribute_octahedron_transform: AttributeOctahedronTransform,
}

impl Default for SequentialNormalAttributeDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl SequentialNormalAttributeDecoder {
    pub fn new() -> Self {
        Self {
            base: SequentialIntegerAttributeDecoder::new(),
            attribute_octahedron_transform: AttributeOctahedronTransform::new(-1),
        }
    }

    pub fn init(&mut self, decoder: &PointCloudDecoder, point_cloud: &PointCloud, attribute_id: i32) -> Status {
        if !self.base.init(decoder, attribute_id) {
            return Err(DracoError::DracoError("Failed to init base".to_string()));
        }
        
        let attribute = point_cloud.attribute(attribute_id);
        if attribute.num_components() != 3 {
            return Err(DracoError::InvalidParameter("Attribute must have 3 components".to_string()));
        }
        
        Ok(())
    }

    pub fn decode_data_needed_by_portable_transform(&mut self, _point_cloud: &mut PointCloud, buffer: &mut DecoderBuffer) -> Status {
        let quantization_bits: u8;
        if let Ok(val) = buffer.decode::<u8>() {
            quantization_bits = val;
        } else {
            return Err(DracoError::BitstreamVersionUnsupported);
        }
        self.attribute_octahedron_transform.set_parameters(quantization_bits as i32);
        Ok(())
    }

    pub fn decode_values(
        &mut self,
        point_cloud: &mut PointCloud,
        point_ids: &[PointIndex],
        buffer: &mut DecoderBuffer,
        corner_table: Option<&CornerTable>,
        data_to_corner_map: Option<&[u32]>,
    ) -> Status {
        // Decode quantization bits if not initialized
        if !self.attribute_octahedron_transform.is_initialized() {
             let quantization_bits: u8 = match buffer.decode_u8() {
                 Ok(v) => v,
                 Err(_) => return Err(DracoError::DracoError("Failed to decode quantization bits".to_string())),
             };
             self.attribute_octahedron_transform.set_parameters(quantization_bits as i32);
        }

        // Create portable attribute
        let mut portable_attribute = crate::geometry_attribute::PointAttribute::new();
        portable_attribute.init(
            crate::geometry_attribute::GeometryAttributeType::Generic,
            2,
            DataType::Uint32,
            false,
            point_ids.len()
        );
        
        // 1. Create prediction scheme
        let transform = PredictionSchemeNormalOctahedronCanonicalizedDecodingTransform::new();
        let prediction_scheme = Box::new(PredictionSchemeDeltaDecoder::new(transform));
        
        self.base.set_prediction_scheme(prediction_scheme);
        
        if !self.base.decode_values(point_cloud, point_ids, buffer, corner_table, data_to_corner_map, Some(&mut portable_attribute)) {
            return Err(DracoError::DracoError("Failed to decode values".to_string()));
        }
        
        // 2. Convert portable attribute to original attribute
        
        // Transform back to original attribute
        let attribute_id = self.base.attribute_id();
        let attribute = point_cloud.attribute_mut(attribute_id);
        
        if !self.attribute_octahedron_transform.inverse_transform_attribute(&portable_attribute, attribute) {
             return Err(DracoError::DracoError("Failed to inverse transform attribute".to_string()));
        }
        
        Ok(())
    }
}
