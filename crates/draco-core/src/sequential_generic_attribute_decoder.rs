use crate::point_cloud::PointCloud;
use crate::decoder_buffer::DecoderBuffer;
use crate::geometry_indices::PointIndex;
use crate::status::{Status, DracoError};
use crate::sequential_attribute_decoder::SequentialAttributeDecoder;
use crate::point_cloud_decoder::PointCloudDecoder;
use crate::draco_types::DataType;

pub struct SequentialGenericAttributeDecoder {
    base: SequentialAttributeDecoder,
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
        let data_type = attribute.data_type();
        let num_points = point_ids.len();
        let data_type_size = data_type.byte_length() as usize;
        
        let total_size = num_points * num_components * data_type_size;
        attribute.buffer_mut().resize(total_size);
        
        let mut offset = 0;
        for _ in 0..num_points {
            for _ in 0..num_components {
                match data_type {
                    DataType::Uint8 => {
                        let val = buffer.decode_u8().map_err(|_| DracoError::DracoError("Failed to decode u8".to_string()))?;
                        attribute.buffer_mut().write(offset, &[val]);
                        offset += 1;
                    },
                    DataType::Int8 => {
                        let val = buffer.decode_u8().map_err(|_| DracoError::DracoError("Failed to decode i8".to_string()))?;
                        attribute.buffer_mut().write(offset, &[val]);
                        offset += 1;
                    },
                    DataType::Uint16 => {
                        let val = buffer.decode_u16().map_err(|_| DracoError::DracoError("Failed to decode u16".to_string()))?;
                        attribute.buffer_mut().write(offset, &val.to_le_bytes());
                        offset += 2;
                    },
                    DataType::Int16 => {
                        let val = buffer.decode_u16().map_err(|_| DracoError::DracoError("Failed to decode i16".to_string()))?;
                        attribute.buffer_mut().write(offset, &val.to_le_bytes());
                        offset += 2;
                    },
                    DataType::Uint32 => {
                        let val = buffer.decode_u32().map_err(|_| DracoError::DracoError("Failed to decode u32".to_string()))?;
                        attribute.buffer_mut().write(offset, &val.to_le_bytes());
                        offset += 4;
                    },
                    DataType::Int32 => {
                        let val = buffer.decode_u32().map_err(|_| DracoError::DracoError("Failed to decode i32".to_string()))?;
                        attribute.buffer_mut().write(offset, &val.to_le_bytes());
                        offset += 4;
                    },
                    DataType::Uint64 => {
                        let val = buffer.decode_u64().map_err(|_| DracoError::DracoError("Failed to decode u64".to_string()))?;
                        attribute.buffer_mut().write(offset, &val.to_le_bytes());
                        offset += 8;
                    },
                    DataType::Int64 => {
                        let val = buffer.decode_u64().map_err(|_| DracoError::DracoError("Failed to decode i64".to_string()))?;
                        attribute.buffer_mut().write(offset, &val.to_le_bytes());
                        offset += 8;
                    },
                    DataType::Float32 => {
                        let val = buffer.decode_f32().map_err(|_| DracoError::DracoError("Failed to decode f32".to_string()))?;
                        attribute.buffer_mut().write(offset, &val.to_le_bytes());
                        offset += 4;
                    },
                    DataType::Float64 => {
                        let val = buffer.decode_f64().map_err(|_| DracoError::DracoError("Failed to decode f64".to_string()))?;
                        attribute.buffer_mut().write(offset, &val.to_le_bytes());
                        offset += 8;
                    },
                    DataType::Bool => {
                        let val = buffer.decode_u8().map_err(|_| DracoError::DracoError("Failed to decode bool".to_string()))?;
                        attribute.buffer_mut().write(offset, &[val]);
                        offset += 1;
                    },
                    _ => return Err(DracoError::DracoError("Unsupported data type".to_string())),
                }
            }
        }
        Ok(())
    }
}
