use crate::attribute_transform::{AttributeTransform, AttributeTransformType};
use crate::attribute_transform_data::AttributeTransformData;
use crate::draco_types::DataType;
use crate::geometry_attribute::PointAttribute;
use crate::geometry_indices::PointIndex;
use crate::normal_compression_utils::OctahedronToolBox;
use crate::status::{DracoError, Status};
#[cfg(feature = "decoder")]
use crate::decoder_buffer::DecoderBuffer;
#[cfg(feature = "encoder")]
use crate::encoder_buffer::EncoderBuffer;

pub struct AttributeOctahedronTransform {
    quantization_bits: i32,
}

impl AttributeOctahedronTransform {
    pub fn new(quantization_bits: i32) -> Self {
        Self { quantization_bits }
    }

    pub fn set_parameters(&mut self, quantization_bits: i32) {
        self.quantization_bits = quantization_bits;
    }

    pub fn is_initialized(&self) -> bool {
        self.quantization_bits != -1
    }

    pub fn quantization_bits(&self) -> i32 {
        self.quantization_bits
    }

    pub fn generate_portable_attribute(
        &self,
        attribute: &PointAttribute,
        point_ids: &[PointIndex],
        num_points: usize,
        target_attribute: &mut PointAttribute,
    ) -> Status {
        if !self.is_initialized() {
            return Err(DracoError::InvalidParameter("Not initialized".to_string()));
        }

        let mut converter = OctahedronToolBox::new();
        if !converter.set_quantization_bits(self.quantization_bits) {
            return Err(DracoError::InvalidParameter("Invalid quantization bits".to_string()));
        }

        let mut att_val = [0.0f32; 3];
        let mut portable_data = Vec::with_capacity(num_points * 2 * 4); // 2 components * 4 bytes

        if !point_ids.is_empty() {
            for &point_id in point_ids {
                let att_val_id = attribute.mapped_index(point_id);
                let offset = att_val_id.0 as usize * attribute.byte_stride() as usize;
                let buffer = attribute.buffer();
                let bytes = &buffer.data()[offset..offset + 12];
                att_val[0] = bytemuck::pod_read_unaligned::<f32>(&bytes[0..4]);
                att_val[1] = bytemuck::pod_read_unaligned::<f32>(&bytes[4..8]);
                att_val[2] = bytemuck::pod_read_unaligned::<f32>(&bytes[8..12]);

                let (s, t) = converter.float_vector_to_quantized_octahedral_coords(&att_val);
                portable_data.extend_from_slice(&s.to_le_bytes());
                portable_data.extend_from_slice(&t.to_le_bytes());
            }
        } else {
            for i in 0..num_points {
                let att_val_id = attribute.mapped_index(PointIndex(i as u32));
                let offset = att_val_id.0 as usize * attribute.byte_stride() as usize;
                let buffer = attribute.buffer();
                let bytes = &buffer.data()[offset..offset + 12];
                att_val[0] = bytemuck::pod_read_unaligned::<f32>(&bytes[0..4]);
                att_val[1] = bytemuck::pod_read_unaligned::<f32>(&bytes[4..8]);
                att_val[2] = bytemuck::pod_read_unaligned::<f32>(&bytes[8..12]);

                let (s, t) = converter.float_vector_to_quantized_octahedral_coords(&att_val);
                portable_data.extend_from_slice(&s.to_le_bytes());
                portable_data.extend_from_slice(&t.to_le_bytes());
            }
        }
        
        target_attribute.buffer_mut().resize(portable_data.len());
        target_attribute.buffer_mut().write(0, &portable_data);

        Ok(())
    }
}

impl AttributeTransform for AttributeOctahedronTransform {
    fn transform_type(&self) -> AttributeTransformType {
        AttributeTransformType::OctahedronTransform
    }

    fn init_from_attribute(&mut self, attribute: &PointAttribute) -> bool {
        if let Some(transform_data) = attribute.attribute_transform_data() {
            if transform_data.transform_type() != AttributeTransformType::OctahedronTransform {
                return false;
            }
            if let Some(bits) = transform_data.get_parameter_value(0) {
                self.quantization_bits = bits;
                return true;
            }
        }
        false
    }

    fn copy_to_attribute_transform_data(&self, out_data: &mut AttributeTransformData) {
        out_data.set_transform_type(AttributeTransformType::OctahedronTransform);
        out_data.append_parameter_value(self.quantization_bits);
    }

    fn transform_attribute(
        &self,
        attribute: &PointAttribute,
        point_ids: &[PointIndex],
        target_attribute: &mut PointAttribute,
    ) -> bool {
        self.generate_portable_attribute(attribute, point_ids, target_attribute.size(), target_attribute).is_ok()
    }

    fn inverse_transform_attribute(
        &self,
        attribute: &PointAttribute,
        target_attribute: &mut PointAttribute,
    ) -> bool {
        if target_attribute.data_type() != DataType::Float32 {
            return false;
        }
        if target_attribute.num_components() != 3 {
            return false;
        }

        let num_points = target_attribute.size();
        let mut converter = OctahedronToolBox::new();
        if !converter.set_quantization_bits(self.quantization_bits) {
            return false;
        }

        let source_buffer = attribute.buffer();
        let target_buffer = target_attribute.buffer_mut();
        
        // Ensure target buffer has enough space
        let target_byte_size = num_points * 3 * 4; // 3 floats * 4 bytes
        target_buffer.resize(target_byte_size);

        let source_data = source_buffer.data();
        // Source data is int32 (s, t) pairs.
        
        for i in 0..num_points {
            let offset = i * 2 * 4; // 2 int32s
            if offset + 8 > source_data.len() {
                return false;
            }
            
            let s = i32::from_le_bytes(source_data[offset..offset+4].try_into().unwrap());
            let t = i32::from_le_bytes(source_data[offset+4..offset+8].try_into().unwrap());
            
            let att_val = converter.quantized_octahedral_coords_to_unit_vector(s, t);
            
            let target_offset = i * 3 * 4;
            // Write floats using bytemuck
            let bytes = &mut target_buffer.data_mut()[target_offset..target_offset + 12];
            bytes[0..4].copy_from_slice(bytemuck::bytes_of(&att_val[0]));
            bytes[4..8].copy_from_slice(bytemuck::bytes_of(&att_val[1]));
            bytes[8..12].copy_from_slice(bytemuck::bytes_of(&att_val[2]));
        }

        true
    }

    #[cfg(feature = "encoder")]
    fn encode_parameters(&self, encoder_buffer: &mut EncoderBuffer) -> bool {
        if self.is_initialized() {
            encoder_buffer.encode(self.quantization_bits as u8);
            true
        } else {
            false
        }
    }

    #[cfg(feature = "decoder")]
    fn decode_parameters(
        &mut self,
        _attribute: &PointAttribute,
        decoder_buffer: &mut DecoderBuffer,
    ) -> bool {
        if let Ok(quantization_bits) = decoder_buffer.decode::<u8>() {
            self.quantization_bits = quantization_bits as i32;
            true
        } else {
            false
        }
    }
    
    fn get_transformed_data_type(&self, _attribute: &PointAttribute) -> DataType {
        DataType::Uint32
    }

    fn get_transformed_num_components(&self, _attribute: &PointAttribute) -> i32 {
        2
    }
}
