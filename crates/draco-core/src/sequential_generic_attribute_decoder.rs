use crate::decoder_buffer::DecoderBuffer;
use crate::geometry_indices::PointIndex;
use crate::point_cloud::PointCloud;
use crate::point_cloud_decoder::PointCloudDecoder;
use crate::sequential_attribute_decoder::SequentialAttributeDecoder;
use crate::status::{DracoError, Status};

pub struct SequentialGenericAttributeDecoder {
    base: SequentialAttributeDecoder,
}

impl Default for SequentialGenericAttributeDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl SequentialGenericAttributeDecoder {
    pub fn new() -> Self {
        Self {
            base: SequentialAttributeDecoder::new(),
        }
    }

    pub fn init(&mut self, decoder: &PointCloudDecoder, attribute_id: i32) -> bool {
        self.base.init(decoder, attribute_id)
    }

    pub fn decode_values(
        &mut self,
        point_cloud: &mut PointCloud,
        point_ids: &[PointIndex],
        buffer: &mut DecoderBuffer,
    ) -> Status {
        let attribute_id = self.base.attribute_id();
        let attribute = point_cloud.attribute_mut(attribute_id);

        let num_components = attribute.num_components() as usize;
        let num_points = point_ids.len();
        let data_type_size = attribute.data_type().byte_length();

        let total_size = num_points
            .checked_mul(num_components)
            .and_then(|size| size.checked_mul(data_type_size))
            .ok_or_else(|| DracoError::DracoError("Generic attribute size overflow".to_string()))?;
        attribute.buffer_mut().try_resize(total_size).map_err(|_| {
            DracoError::DracoError("Failed to allocate generic attribute".to_string())
        })?;

        let bytes = buffer.decode_slice(total_size).map_err(|_| {
            DracoError::DracoError("Failed to decode generic attribute".to_string())
        })?;
        attribute.buffer_mut().data_mut().copy_from_slice(bytes);
        Ok(())
    }
}
