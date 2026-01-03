use std::mem;

pub struct EncoderBuffer {
    buffer: Vec<u8>,
    bit_encoder_active: bool,
    bit_start_pos: usize,
    current_bit_offset: usize,
    version_major: u8,
    version_minor: u8,
    encode_bit_sequence_size: bool,
}

impl Default for EncoderBuffer {
    fn default() -> Self {
        Self {
            buffer: Vec::new(),
            bit_encoder_active: false,
            bit_start_pos: 0,
            current_bit_offset: 0,
            version_major: 0,
            version_minor: 0,
            encode_bit_sequence_size: false,
        }
    }
}

impl EncoderBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_version(&mut self, major: u8, minor: u8) {
        self.version_major = major;
        self.version_minor = minor;
    }

    pub fn version_major(&self) -> u8 {
        self.version_major
    }

    pub fn version_minor(&self) -> u8 {
        self.version_minor
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.bit_encoder_active = false;
        self.current_bit_offset = 0;
    }

    pub fn resize(&mut self, nbytes: usize) {
        self.buffer.resize(nbytes, 0);
    }

    pub fn start_bit_encoding(&mut self, required_bits: usize, encode_size: bool) -> bool {
        if self.bit_encoder_active {
            return false;
        }
        self.encode_bit_sequence_size = encode_size;
        if encode_size {
            // Reserve 8 bytes for the size (will be replaced by varint or fixed 8 bytes later)
            for _ in 0..8 {
                self.buffer.push(0);
            }
        }
        let required_bytes = (required_bits + 7) / 8;
        self.bit_start_pos = self.buffer.len();
        self.buffer.resize(self.bit_start_pos + required_bytes, 0);
        self.bit_encoder_active = true;
        self.current_bit_offset = 0;
        true
    }

    pub fn end_bit_encoding(&mut self) {
        if !self.bit_encoder_active {
            return;
        }
        self.bit_encoder_active = false;
        
        if self.encode_bit_sequence_size {
            let encoded_bits = self.current_bit_offset;
            let encoded_bytes = (encoded_bits + 7) / 8;
            let bitstream_version = ((self.version_major as u16) << 8) | (self.version_minor as u16);
            
            let mut var_size_buffer = Vec::new();
            if bitstream_version >= 0x0202 {
                // Encode size as varint
                let mut v = encoded_bytes as u64;
                loop {
                    let mut byte = (v & 0x7F) as u8;
                    v >>= 7;
                    if v != 0 {
                        byte |= 0x80;
                        var_size_buffer.push(byte);
                    } else {
                        var_size_buffer.push(byte);
                        break;
                    }
                }
            } else {
                // Encode size as fixed 8 bytes
                var_size_buffer.extend_from_slice(&(encoded_bytes as u64).to_le_bytes());
            }
            
            let size_len = var_size_buffer.len();
            let reserved_pos = self.bit_start_pos - 8;
            
            // Move encoded data to its final position
            let src_pos = self.bit_start_pos;
            let dst_pos = reserved_pos + size_len;
            
            if dst_pos != src_pos {
                self.buffer.copy_within(src_pos..src_pos + encoded_bytes, dst_pos);
            }
            
            // Write the size
            self.buffer[reserved_pos..reserved_pos + size_len].copy_from_slice(&var_size_buffer);
            
            // Resize buffer to final size
            self.buffer.resize(dst_pos + encoded_bytes, 0);
        } else {
            // Just resize to actual encoded bytes
            let encoded_bytes = (self.current_bit_offset + 7) / 8;
            self.buffer.resize(self.bit_start_pos + encoded_bytes, 0);
        }
    }

    pub fn encode_least_significant_bits32(&mut self, nbits: u32, value: u32) -> bool {
        if !self.bit_encoder_active {
            return false;
        }
        for i in 0..nbits {
            let bit = (value >> i) & 1;
            self.put_bit(bit);
        }
        true
    }

    fn put_bit(&mut self, bit: u32) {
        let total_bit_offset = self.current_bit_offset;
        let byte_offset = self.bit_start_pos + total_bit_offset / 8;
        let bit_shift = total_bit_offset % 8;
        
        if byte_offset < self.buffer.len() {
            if bit != 0 {
                self.buffer[byte_offset] |= 1 << bit_shift;
            } else {
                self.buffer[byte_offset] &= !(1 << bit_shift);
            }
        }
        self.current_bit_offset += 1;
    }

    pub fn encode<T>(&mut self, data: T) -> bool {
        if self.bit_encoder_active {
            return false;
        }
        let size = mem::size_of::<T>();
        let ptr = &data as *const T as *const u8;
        let slice = unsafe { std::slice::from_raw_parts(ptr, size) };
        self.buffer.extend_from_slice(slice);
        true
    }

    pub fn encode_data(&mut self, data: &[u8]) -> bool {
        if self.bit_encoder_active {
            return false;
        }
        self.buffer.extend_from_slice(data);
        true
    }

    pub fn encode_u8(&mut self, val: u8) {
        self.buffer.push(val);
    }

    pub fn encode_u16(&mut self, val: u16) {
        self.buffer.extend_from_slice(&val.to_le_bytes());
    }

    pub fn encode_u32(&mut self, val: u32) {
        self.buffer.extend_from_slice(&val.to_le_bytes());
    }

    pub fn encode_u64(&mut self, val: u64) {
        self.buffer.extend_from_slice(&val.to_le_bytes());
    }

    pub fn encode_varint<T: Into<u64>>(&mut self, val: T) {
        let mut v = val.into();
        loop {
            let mut byte = (v & 0x7F) as u8;
            v >>= 7;
            if v != 0 {
                byte |= 0x80;
                self.buffer.push(byte);
            } else {
                self.buffer.push(byte);
                break;
            }
        }
    }

    /// Draco-compatible signed varint (ConvertSignedIntToSymbol + unsigned varint).
    pub fn encode_varint_signed_i32(&mut self, val: i32) {
        let symbol: u32 = if val >= 0 {
            (val as u32) << 1
        } else {
            let mapped = (-(val + 1)) as u32;
            (mapped << 1) | 1
        };
        self.encode_varint(symbol as u64);
    }
    
    pub fn data(&self) -> &[u8] {
        &self.buffer
    }
    
    pub fn size(&self) -> usize {
        self.buffer.len()
    }
}
