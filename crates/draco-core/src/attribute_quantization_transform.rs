use crate::attribute_transform::{AttributeTransform, AttributeTransformType};
use crate::attribute_transform_data::AttributeTransformData;
#[cfg(feature = "decoder")]
use crate::decoder_buffer::DecoderBuffer;
use crate::draco_types::DataType;
#[cfg(feature = "encoder")]
use crate::encoder_buffer::EncoderBuffer;
use crate::geometry_attribute::PointAttribute;
use crate::geometry_indices::PointIndex;
use crate::quantization_utils::{Dequantizer, Quantizer};

pub struct AttributeQuantizationTransform {
    quantization_bits: i32,
    min_values: Vec<f32>,
    range: f32,
}

impl Default for AttributeQuantizationTransform {
    fn default() -> Self {
        Self {
            quantization_bits: -1,
            min_values: Vec::new(),
            range: 0.0,
        }
    }
}

impl AttributeQuantizationTransform {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_parameters(&mut self, quantization_bits: i32, min_values: &[f32], range: f32) -> bool {
        if !(1..=31).contains(&quantization_bits) {
            return false;
        }
        self.quantization_bits = quantization_bits;
        self.min_values = min_values.to_vec();
        self.range = range;
        true
    }

    pub fn compute_parameters(&mut self, attribute: &PointAttribute, quantization_bits: i32) -> bool {
        if !(1..=31).contains(&quantization_bits) {
            return false;
        }
        self.quantization_bits = quantization_bits;
        let num_components = attribute.num_components() as usize;
        self.min_values = vec![f32::MAX; num_components];
        let mut max_values = vec![f32::MIN; num_components];

        let num_entries = attribute.size();
        
        if attribute.data_type() != DataType::Float32 {
            return false;
        }

        let buffer = attribute.buffer();
        let byte_stride = attribute.byte_stride() as usize;
        
        for i in 0..num_entries {
            let offset = i * byte_stride;
            // Read num_components floats
            for c in 0..num_components {
                let ptr = &buffer.data()[offset + c * 4] as *const u8 as *const f32;
                let val = unsafe { ptr.read_unaligned() };
                
                if val < self.min_values[c] {
                    self.min_values[c] = val;
                }
                if val > max_values[c] {
                    max_values[c] = val;
                }
            }
        }

        self.range = 0.0;
        for c in 0..num_components {
            let diff = max_values[c] - self.min_values[c];
            if diff > self.range {
                self.range = diff;
            }
        }
        
        // Adjust range if it is 0?
        if self.range == 0.0 {
            self.range = 1.0;
        }

        true
    }

    fn generate_portable_attribute(&self, attribute: &PointAttribute, point_ids: &[PointIndex], target_attribute: &mut PointAttribute) {
        if self.quantization_bits < 1 || self.quantization_bits > 31 {
            // Invalid state; caller should have initialized parameters.
            return;
        }
        let num_points = if point_ids.is_empty() { attribute.size() } else { point_ids.len() };
        let num_components = attribute.num_components();
        
        target_attribute.init(
            attribute.attribute_type(),
            num_components,
            DataType::Uint32, // Quantized values are usually stored as integers
            false,
            num_points,
        );

        // quantization_bits is allowed up to 31. Use a wider type to avoid
        // overflowing signed shifts (e.g. 1 << 31 on i32).
        let max_quantized_value: i32 = ((1u64 << (self.quantization_bits as u32)) - 1) as i32;
        let mut quantizer = Quantizer::new();
        quantizer.init(self.range, max_quantized_value);

        let src_buffer = attribute.buffer();
        let src_stride = attribute.byte_stride() as usize;
        let dst_stride = target_attribute.byte_stride() as usize;
        let dst_buffer = target_attribute.buffer_mut();

        for i in 0..num_points {
            // Use mapped_index to get the correct AttributeValueIndex, matching C++ behavior
            let point_idx = if point_ids.is_empty() { PointIndex(i as u32) } else { point_ids[i] };
            let att_val_idx = attribute.mapped_index(point_idx);
            let src_offset = att_val_idx.0 as usize * src_stride;
            let dst_offset = i * dst_stride;

            for c in 0..num_components as usize {
                let ptr = &src_buffer.data()[src_offset + c * 4] as *const u8 as *const f32;
                let mut val = unsafe { ptr.read_unaligned() };

                val -= self.min_values[c];
                let q_val = quantizer.quantize_float(val);
                
                let q_val_u32 = q_val as u32;
                let dst_ptr = &mut dst_buffer.data_mut()[dst_offset + c * 4] as *mut u8 as *mut u32;
                unsafe { dst_ptr.write_unaligned(q_val_u32); }
            }
        }
    }
}

impl AttributeTransform for AttributeQuantizationTransform {
    fn transform_type(&self) -> AttributeTransformType {
        AttributeTransformType::QuantizationTransform
    }

    fn init_from_attribute(&mut self, attribute: &PointAttribute) -> bool {
        if let Some(data) = attribute.attribute_transform_data() {
            if data.transform_type() != AttributeTransformType::QuantizationTransform {
                return false;
            }
            let mut byte_offset = 0;
            if let Some(bits) = data.get_parameter_value::<i32>(byte_offset) {
                self.quantization_bits = bits;
                byte_offset += 4;
            } else { return false; }

            let num_components = attribute.num_components() as usize;
            self.min_values.resize(num_components, 0.0);
            for i in 0..num_components {
                if let Some(val) = data.get_parameter_value::<f32>(byte_offset) {
                    self.min_values[i] = val;
                    byte_offset += 4;
                } else { return false; }
            }

            if let Some(range) = data.get_parameter_value::<f32>(byte_offset) {
                self.range = range;
            } else { return false; }

            true
        } else {
            false
        }
    }

    fn copy_to_attribute_transform_data(&self, out_data: &mut AttributeTransformData) {
        out_data.set_transform_type(AttributeTransformType::QuantizationTransform);
        out_data.append_parameter_value(self.quantization_bits);
        for &val in &self.min_values {
            out_data.append_parameter_value(val);
        }
        out_data.append_parameter_value(self.range);
    }

    fn transform_attribute(
        &self,
        attribute: &PointAttribute,
        point_ids: &[PointIndex],
        target_attribute: &mut PointAttribute,
    ) -> bool {
        self.generate_portable_attribute(attribute, point_ids, target_attribute);
        true
    }

    fn inverse_transform_attribute(
        &self,
        attribute: &PointAttribute,
        target_attribute: &mut PointAttribute,
    ) -> bool {
        if target_attribute.data_type() != DataType::Float32 {
            return false;
        }

        if self.quantization_bits < 1 || self.quantization_bits > 31 {
            return false;
        }

        // quantization_bits is allowed up to 31. Use a wider type to avoid
        // overflowing signed shifts (e.g. 1 << 31 on i32).
        let max_quantized_value: i32 = ((1u64 << (self.quantization_bits as u32)) - 1) as i32;
        let mut dequantizer = Dequantizer::new();
        if !dequantizer.init(self.range, max_quantized_value) {
            return false;
        }

        let num_components = target_attribute.num_components() as usize;
        let num_values = target_attribute.size();
        
        let src_buffer = attribute.buffer();
        let dst_stride = target_attribute.byte_stride() as usize;
        let dst_buffer = target_attribute.buffer_mut();
        
        let src_stride = attribute.byte_stride() as usize;

        for i in 0..num_values {
            let src_offset = i * src_stride;
            let dst_offset = i * dst_stride;

            for c in 0..num_components {
                let ptr = &src_buffer.data()[src_offset + c * 4] as *const u8 as *const i32;
                let q_val = unsafe { ptr.read_unaligned() };

                let mut val = dequantizer.dequantize_float(q_val);
                val += self.min_values[c];

                let dst_ptr = &mut dst_buffer.data_mut()[dst_offset + c * 4] as *mut u8 as *mut f32;
                unsafe { dst_ptr.write_unaligned(val); }
            }
        }

        true
    }

    #[cfg(feature = "encoder")]
    fn encode_parameters(&self, encoder_buffer: &mut EncoderBuffer) -> bool {
        for &val in &self.min_values {
            encoder_buffer.encode(val);
        }
        encoder_buffer.encode(self.range);
        encoder_buffer.encode_u8(self.quantization_bits as u8);
        true
    }

    #[cfg(feature = "decoder")]
    fn decode_parameters(
        &mut self,
        attribute: &PointAttribute,
        decoder_buffer: &mut DecoderBuffer,
    ) -> bool {
        let num_components = attribute.num_components() as usize;
        
        self.min_values.resize(num_components, 0.0);
        for i in 0..num_components {
            if let Ok(val) = decoder_buffer.decode::<f32>() {
                self.min_values[i] = val;
            } else { return false; }
        }

        if let Ok(range) = decoder_buffer.decode::<f32>() {
            self.range = range;
        } else { return false; }

        if let Ok(bits) = decoder_buffer.decode_u8() {
            self.quantization_bits = bits as i32;
        } else {
            return false;
        }

        if self.quantization_bits < 1 || self.quantization_bits > 31 {
            return false;
        }

        true
    }

    fn get_transformed_data_type(&self, _attribute: &PointAttribute) -> DataType {
        DataType::Uint32
    }

    fn get_transformed_num_components(&self, attribute: &PointAttribute) -> i32 {
        attribute.num_components() as i32
    }
}
