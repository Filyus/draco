use std::io::{self, Write};

#[derive(Debug, Default, Clone)]
pub struct DataBufferDescriptor {
    pub buffer_id: i64,
    pub buffer_update_count: i64,
}

#[derive(Debug, Default, Clone)]
pub struct DataBuffer {
    data: Vec<u8>,
    descriptor: DataBufferDescriptor,
}

impl DataBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, data: &[u8], offset: Option<usize>) {
        let offset = offset.unwrap_or(0);
        let end = offset + data.len();
        
        if end > self.data.len() {
            self.data.resize(end, 0);
        }
        
        self.data[offset..end].copy_from_slice(data);
        self.descriptor.buffer_update_count += 1;
    }

    pub fn resize(&mut self, new_size: usize) {
        self.data.resize(new_size, 0);
    }

    pub fn write_data_to_stream<W: Write>(&self, stream: &mut W) -> io::Result<()> {
        stream.write_all(&self.data)
    }

    pub fn read(&self, byte_pos: usize, out_data: &mut [u8]) {
        let len = out_data.len();
        out_data.copy_from_slice(&self.data[byte_pos..byte_pos + len]);
    }

    pub fn write(&mut self, byte_pos: usize, in_data: &[u8]) {
        let len = in_data.len();
        self.data[byte_pos..byte_pos + len].copy_from_slice(in_data);
    }

    pub fn copy(&mut self, dst_offset: usize, src_buf: &DataBuffer, src_offset: usize, size: usize) {
        let src_slice = &src_buf.data[src_offset..src_offset + size];
        if dst_offset + size > self.data.len() {
            self.data.resize(dst_offset + size, 0);
        }
        self.data[dst_offset..dst_offset + size].copy_from_slice(src_slice);
    }

    pub fn set_update_count(&mut self, count: i64) {
        self.descriptor.buffer_update_count = count;
    }

    pub fn update_count(&self) -> i64 {
        self.descriptor.buffer_update_count
    }

    pub fn data_size(&self) -> usize {
        self.data.len()
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn buffer_id(&self) -> i64 {
        self.descriptor.buffer_id
    }

    pub fn set_buffer_id(&mut self, buffer_id: i64) {
        self.descriptor.buffer_id = buffer_id;
    }
}
