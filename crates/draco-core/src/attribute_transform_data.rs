use crate::attribute_transform::AttributeTransformType;
use crate::data_buffer::DataBuffer;
use std::mem;

#[derive(Debug, Clone)]
pub struct AttributeTransformData {
    transform_type: AttributeTransformType,
    buffer: DataBuffer,
}

impl Default for AttributeTransformData {
    fn default() -> Self {
        Self {
            transform_type: AttributeTransformType::InvalidTransform,
            buffer: DataBuffer::new(),
        }
    }
}

impl AttributeTransformData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn transform_type(&self) -> AttributeTransformType {
        self.transform_type
    }

    pub fn set_transform_type(&mut self, transform_type: AttributeTransformType) {
        self.transform_type = transform_type;
    }

    pub fn get_parameter_value<T: Copy>(&self, byte_offset: usize) -> Option<T> {
        let size = mem::size_of::<T>();
        if byte_offset + size > self.buffer.data_size() {
            return None;
        }
        // We need a way to read T from buffer.
        // DataBuffer::read takes &mut [u8].
        // We can read into a temporary buffer and then transmute/read_unaligned.
        // Or just use unsafe.
        
        // Let's use a safer approach if possible, or just unsafe.
        // Since T is Copy, we can assume it's POD-like for this context.
        let mut val: T = unsafe { mem::zeroed() };
        let ptr = &mut val as *mut T as *mut u8;
        let slice = unsafe { std::slice::from_raw_parts_mut(ptr, size) };
        self.buffer.read(byte_offset, slice);
        Some(val)
    }

    pub fn set_parameter_value<T: Copy>(&mut self, byte_offset: usize, in_data: T) {
        let size = mem::size_of::<T>();
        if byte_offset + size > self.buffer.data_size() {
            self.buffer.resize(byte_offset + size);
        }
        let ptr = &in_data as *const T as *const u8;
        let slice = unsafe { std::slice::from_raw_parts(ptr, size) };
        self.buffer.write(byte_offset, slice);
    }

    pub fn append_parameter_value<T: Copy>(&mut self, in_data: T) {
        self.set_parameter_value(self.buffer.data_size(), in_data);
    }
}
