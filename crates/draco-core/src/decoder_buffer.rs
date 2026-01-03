use std::mem;
use crate::version::DEFAULT_MESH_VERSION;

pub struct DecoderBuffer<'a> {
    data: &'a [u8],
    pos: usize,
    bit_decoder_active: bool,
    bit_start_pos: usize,
    current_bit_offset: usize,
    bit_stream_end_pos: usize,
    bit_sequence_size_known: bool,
    version_major: u8,
    version_minor: u8,
}

impl<'a> DecoderBuffer<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            bit_decoder_active: false,
            bit_start_pos: 0,
            current_bit_offset: 0,
            bit_stream_end_pos: 0,
            bit_sequence_size_known: false,
            // Default to latest mesh version to match encoder output format
            version_major: DEFAULT_MESH_VERSION.0,
            version_minor: DEFAULT_MESH_VERSION.1,
        }
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

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn set_position(&mut self, pos: usize) -> Result<(), ()> {
        if self.bit_decoder_active {
            return Err(());
        }
        if pos > self.data.len() {
            return Err(());
        }
        self.pos = pos;
        Ok(())
    }

    pub fn remaining_size(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    pub fn peek_bytes(&self, len: usize) -> Vec<u8> {
        let end = std::cmp::min(self.pos + len, self.data.len());
        self.data[self.pos..end].to_vec()
    }

    pub fn start_bit_decoding(&mut self, decode_size: bool) -> Result<u64, ()> {
        if self.bit_decoder_active {
            return Err(());
        }
        let bitstream_version = ((self.version_major as u16) << 8) | (self.version_minor as u16);
        // Draco stores the bit-sequence size in BYTES (not bits) when |decode_size| is true.
        let mut size_bytes: u64 = 0;
        if decode_size {
            if bitstream_version < 0x0202 {
                size_bytes = self.decode_u64().map_err(|_| ())?;
            } else {
                size_bytes = self.decode_varint().map_err(|_| ())?;
            }
        }
        
        self.bit_start_pos = self.pos;
        self.bit_decoder_active = true;
        self.current_bit_offset = 0;
        self.bit_sequence_size_known = decode_size;
        
        if decode_size {
             self.bit_stream_end_pos = self.bit_start_pos + size_bytes as usize;
        } else {
             // If size is not encoded, we assume the rest of the buffer?
             // Or caller knows?
             // C++ implementation:
             // if (decode_size) ...
             // else bit_decoder_.Init(..., data_size_ - pos_);
             self.bit_stream_end_pos = self.data.len();
        }

        Ok(size_bytes)
    }

    pub fn end_bit_decoding(&mut self) {
        self.bit_decoder_active = false;
        // Draco behavior:
        // - When decoding with size known, the caller typically skips by the stored byte size.
        // - When decoding without size, advance by the number of decoded bits (rounded up).
        if self.bit_sequence_size_known {
            self.pos = self.bit_stream_end_pos;
        } else {
            let bytes_consumed = (self.current_bit_offset + 7) / 8;
            self.pos = self.bit_start_pos + bytes_consumed;
        }
    }

    pub fn decode_least_significant_bits32(&mut self, nbits: u32) -> Result<u32, ()> {
        if !self.bit_decoder_active {
            return Err(());
        }
        let mut value = 0;
        for i in 0..nbits {
            let bit = self.get_bit()?;
            value |= bit << i;
        }
        Ok(value)
    }

    fn get_bit(&mut self) -> Result<u32, ()> {
        let total_bit_offset = self.current_bit_offset;
        let byte_offset = self.bit_start_pos + total_bit_offset / 8;
        let bit_shift = total_bit_offset % 8;

        if byte_offset < self.bit_stream_end_pos && byte_offset < self.data.len() {
            let bit = (self.data[byte_offset] >> bit_shift) & 1;
            self.current_bit_offset += 1;
            Ok(bit as u32)
        } else {
            Err(())
        }
    }

    pub fn decode<T: Copy>(&mut self) -> Result<T, ()> {
        if self.bit_decoder_active {
            return Err(());
        }
        let size = mem::size_of::<T>();
        if self.pos + size > self.data.len() {
            return Err(());
        }

        // Unsafe copy to T
        let ptr = self.data[self.pos..].as_ptr() as *const T;
        let val = unsafe { ptr.read_unaligned() };
        self.pos += size;
        Ok(val)
    }

    pub fn decode_u8(&mut self) -> Result<u8, ()> {
        self.decode::<u8>()
    }

    pub fn decode_u16(&mut self) -> Result<u16, ()> {
        let mut bytes = [0u8; 2];
        self.decode_bytes(&mut bytes)?;
        Ok(u16::from_le_bytes(bytes))
    }

    pub fn decode_u32(&mut self) -> Result<u32, ()> {
        let mut bytes = [0u8; 4];
        self.decode_bytes(&mut bytes)?;
        Ok(u32::from_le_bytes(bytes))
    }

    pub fn decode_u64(&mut self) -> Result<u64, ()> {
        let mut bytes = [0u8; 8];
        self.decode_bytes(&mut bytes)?;
        Ok(u64::from_le_bytes(bytes))
    }

    pub fn decode_f32(&mut self) -> Result<f32, ()> {
        let mut bytes = [0u8; 4];
        self.decode_bytes(&mut bytes)?;
        Ok(f32::from_le_bytes(bytes))
    }

    pub fn decode_f64(&mut self) -> Result<f64, ()> {
        let mut bytes = [0u8; 8];
        self.decode_bytes(&mut bytes)?;
        Ok(f64::from_le_bytes(bytes))
    }

    pub fn decode_string(&mut self) -> Result<String, ()> {
        let mut bytes = Vec::new();
        loop {
            let b = self.decode_u8()?;
            if b == 0 {
                break;
            }
            bytes.push(b);
        }
        String::from_utf8(bytes).map_err(|_| ())
    }

    pub fn decode_bytes(&mut self, out: &mut [u8]) -> Result<(), ()> {
        let size = out.len();
        if self.pos + size > self.data.len() {
            return Err(());
        }
        out.copy_from_slice(&self.data[self.pos..self.pos + size]);
        self.pos += size;
        Ok(())
    }

    pub fn decode_varint(&mut self) -> Result<u64, ()> {
        let mut val = 0u64;
        let mut shift = 0;
        loop {
            let b = self.decode_u8()?;
            val |= ((b & 0x7F) as u64) << shift;
            if (b & 0x80) == 0 {
                break;
            }
            shift += 7;
            if shift >= 64 {
                return Err(());
            }
        }
        Ok(val)
    }

    /// Draco-compatible signed varint (unsigned varint + ConvertSymbolToSignedInt).
    pub fn decode_varint_signed_i32(&mut self) -> Result<i32, ()> {
        let symbol = self.decode_varint()? as u32;
        let is_positive = (symbol & 1) == 0;
        let v = symbol >> 1;
        if is_positive {
            Ok(v as i32)
        } else {
            Ok(-(v as i32) - 1)
        }
    }

    pub fn remaining_data(&self) -> &'a [u8] {
        &self.data[self.pos..]
    }

    pub fn advance(&mut self, n: usize) {
        self.pos += n;
    }

    pub fn decode_slice(&mut self, size: usize) -> Result<&'a [u8], ()> {
        if self.pos + size > self.data.len() {
            return Err(());
        }
        let slice = &self.data[self.pos..self.pos + size];
        self.pos += size;
        Ok(slice)
    }
}
