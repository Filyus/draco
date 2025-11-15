// Copyright 2022 The Draco Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::mem;
use std::ptr;

/// Decoder buffer for Draco deserialization with memory-safe parsing
/// Supports both byte-aligned and bit-level decoding
#[derive(Debug, Clone)]
pub struct DecoderBuffer {
    /// Input data (borrowed, doesn't own the data)
    data: *const u8,

    /// Total size of the data
    data_size: usize,

    /// Current parsing position
    pos: usize,

    /// Bit decoder for bit-level operations
    bit_decoder: Option<BitDecoder>,

    /// Whether bit decoder is active
    bit_mode: bool,

    /// Bitstream version for compatibility
    bitstream_version: u16,
}

/// Bit decoder for variable-length bit sequences
#[derive(Debug, Clone)]
pub struct BitDecoder {
    /// Current bit offset within the buffer
    bit_offset: usize,

    /// End of the buffer
    bit_buffer_end: *const u8,

    /// Start of the buffer
    bit_buffer: *const u8,
}

impl DecoderBuffer {
    /// Create a new empty decoder buffer
    pub fn new() -> Self {
        Self {
            data: std::ptr::null(),
            data_size: 0,
            pos: 0,
            bit_decoder: None,
            bit_mode: false,
            bitstream_version: 0,
        }
    }

    /// Initialize buffer with data
    ///
    /// # Safety
    /// The caller must ensure that `data` remains valid for the lifetime of the decoder
    pub unsafe fn init(&mut self, data: *const u8, data_size: usize) {
        self.data = data;
        self.data_size = data_size;
        self.pos = 0;
        self.bit_decoder = None;
        self.bit_mode = false;
    }

    /// Initialize buffer with data and bitstream version
    ///
    /// # Safety
    /// The caller must ensure that `data` remains valid for the lifetime of the decoder
    pub unsafe fn init_with_version(&mut self, data: *const u8, data_size: usize, version: u16) {
        self.init(data, data_size);
        self.bitstream_version = version;
    }

    /// Start decoding a bit sequence
    ///
    /// # Arguments
    /// * `decode_size` - Whether to decode the size of the bit sequence
    /// * `out_size` - Output for the decoded size (if decode_size is true)
    pub fn start_bit_decoding(&mut self, decode_size: bool, out_size: &mut u64) -> Result<(), &'static str> {
        if self.bit_mode {
            return Err("Bit decoder already active");
        }

        let mut bit_decoder = BitDecoder::new();
        let mut start_pos = self.pos;

        // Decode size if requested
        if decode_size {
            if self.pos + 4 > self.data_size {
                return Err("Not enough data to decode bit sequence size");
            }

            let size_bytes = unsafe { std::slice::from_raw_parts(self.data.add(self.pos), 4) };
            let bit_count = u32::from_le_bytes([size_bytes[0], size_bytes[1], size_bytes[2], size_bytes[3]]);
            *out_size = bit_count as u64;
            start_pos += 4;
        }

        // Initialize bit decoder with buffer
        bit_decoder.bit_buffer = unsafe { self.data.add(start_pos) };
        bit_decoder.bit_buffer_end = unsafe { self.data.add(self.data_size) };
        bit_decoder.bit_offset = 0;

        self.bit_decoder = Some(bit_decoder);
        self.bit_mode = true;

        Ok(())
    }

    /// End bit decoding and return to byte-aligned mode
    pub fn end_bit_decoding(&mut self) {
        if let Some(ref bit_decoder) = self.bit_decoder {
            // Calculate the number of bytes consumed by bit operations
            let bits_consumed = bit_decoder.bit_offset();
            let bytes_consumed = (bits_consumed + 7) / 8;

            // Update position to after the bit data
            let bit_data_start = bit_decoder.bit_buffer;
            let bit_data_end = bit_decoder.bit_buffer_end;
            let consumed_bytes = unsafe { bit_data_end.offset_from(bit_data_start) } as usize;
            self.pos += (self.data_size - consumed_bytes) + bytes_consumed;

            self.bit_decoder = None;
            self.bit_mode = false;
        }
    }

    /// Peeks at data without advancing the position
    pub fn peek_slice(&self, output: &mut [u8]) -> Result<(), crate::error::Status> {
        if output.is_empty() {
            return Ok(());
        }

        let end_pos = self.pos.checked_add(output.len())
            .ok_or_else(|| crate::error::Status::new(crate::error::ErrorCode::DracoError, "Position overflow"))?;

        if end_pos > self.data_size {
            return Err(crate::error::Status::new(crate::error::ErrorCode::DracoError, "Buffer overflow"));
        }

        unsafe {
            let src = self.data.add(self.pos);
            ptr::copy_nonoverlapping(src, output.as_mut_ptr(), output.len());
        }

        Ok(())
    }

    /// Decodes data and advances the position
    pub fn decode_slice(&mut self, output: &mut [u8]) -> Result<(), crate::error::Status> {
        self.peek_slice(output)?;
        self.pos += output.len();
        Ok(())
    }

    /// Decode least significant bits from the current position
    ///
    /// # Arguments
    /// * `nbits` - Number of bits to decode (1-32)
    pub fn decode_least_significant_bits_32(&mut self, nbits: u8) -> Result<u32, &'static str> {
        if nbits > 32 {
            return Err("Cannot decode more than 32 bits");
        }

        if let Some(ref mut bit_decoder) = self.bit_decoder {
            bit_decoder.get_bits(nbits)
        } else {
            Err("Bit decoder not active")
        }
    }

    /// Decode arbitrary data type (byte-aligned)
    ///
    /// # Safety
    /// This function uses unsafe code to convert bytes to any type.
    /// Ensure the type is POD (plain old data) and doesn't contain references.
    pub unsafe fn decode_type<T>(&mut self) -> Result<T, &'static str> {
        if self.bit_mode {
            return Err("Cannot decode byte-aligned data during bit decoding");
        }

        let size = mem::size_of::<T>();
        if self.pos + size > self.data_size {
            return Err("Buffer overflow during type decode");
        }

        let value = unsafe {
            let ptr = self.data.add(self.pos) as *const T;
            ptr.read_unaligned()
        };

        self.pos += size;
        Ok(value)
    }

    /// Decode raw bytes (byte-aligned)
    pub fn decode(&mut self, out_data: &mut [u8]) -> Result<(), &'static str> {
        if self.bit_mode {
            return Err("Cannot decode byte-aligned data during bit decoding");
        }

        let size = out_data.len();
        if self.pos + size > self.data_size {
            return Err("Buffer overflow during byte decode");
        }

        unsafe {
            let src = self.data.add(self.pos);
            let dst = out_data.as_mut_ptr();
            std::ptr::copy_nonoverlapping(src, dst, size);
        }

        self.pos += size;
        Ok(())
    }

    /// Peek at data without advancing position
    ///
    /// # Safety
    /// This function uses unsafe code to convert bytes to any type.
    /// Ensure the type is POD (plain old data) and doesn't contain references.
    pub unsafe fn peek_type<T>(&self) -> Result<T, &'static str> {
        let size = mem::size_of::<T>();
        if self.pos + size > self.data_size {
            return Err("Buffer overflow during type peek");
        }

        let value = unsafe {
            let ptr = self.data.add(self.pos) as *const T;
            ptr.read_unaligned()
        };

        Ok(value)
    }

    /// Peek at raw bytes without advancing position
    pub fn peek(&self, out_data: &mut [u8]) -> Result<(), &'static str> {
        let size = out_data.len();
        if self.pos + size > self.data_size {
            return Err("Buffer overflow during byte peek");
        }

        unsafe {
            let src = self.data.add(self.pos);
            let dst = out_data.as_mut_ptr();
            std::ptr::copy_nonoverlapping(src, dst, size);
        }

        Ok(())
    }

    /// Advance position by specified number of bytes
    pub fn advance(&mut self, bytes: usize) -> Result<(), &'static str> {
        if self.pos + bytes > self.data_size {
            return Err("Buffer overflow during advance");
        }
        self.pos += bytes;
        Ok(())
    }

    /// Set parsing position to specific offset
    pub fn start_decoding_from(&mut self, offset: usize) -> Result<(), &'static str> {
        if offset > self.data_size {
            return Err("Position beyond buffer size");
        }
        self.pos = offset;
        Ok(())
    }

    /// Set bitstream version
    pub fn set_bitstream_version(&mut self, version: u16) {
        self.bitstream_version = version;
    }

    /// Get current data head (position + base)
    pub fn data_head(&self) -> *const u8 {
        unsafe { self.data.add(self.pos) }
    }

    /// Get remaining size in bytes
    pub fn remaining_size(&self) -> usize {
        self.data_size - self.pos
    }

    /// Get decoded size in bytes
    pub fn decoded_size(&self) -> usize {
        self.pos
    }

    /// Check if bit decoder is active
    pub fn bit_decoder_active(&self) -> bool {
        self.bit_mode
    }

    /// Get bitstream version
    pub fn bitstream_version(&self) -> u16 {
        self.bitstream_version
    }

    /// Get total data size
    pub fn data_size(&self) -> usize {
        self.data_size
    }

    /// Get current position
    pub fn position(&self) -> usize {
        self.pos
    }
}

impl BitDecoder {
    /// Create a new bit decoder
    pub fn new() -> Self {
        Self {
            bit_offset: 0,
            bit_buffer_end: std::ptr::null(),
            bit_buffer: std::ptr::null(),
        }
    }

    /// Reset the bit decoder with new buffer
    ///
    /// # Safety
    /// The caller must ensure that `b` remains valid for the lifetime of the decoder
    pub fn reset(&mut self, b: *const u8, s: usize) {
        self.bit_buffer = b;
        self.bit_buffer_end = unsafe { b.add(s) };
        self.bit_offset = 0;
    }

    /// Get current bit offset
    pub fn bit_offset(&self) -> usize {
        self.bit_offset
    }

    /// Get available bits
    pub fn available_bits(&self) -> usize {
        let buffer_size = unsafe { self.bit_buffer_end.offset_from(self.bit_buffer) } as usize;
        (buffer_size * 8) - self.bit_offset
    }

    /// Ensure that at least k bits are available for reading
    /// This is an optimization that can be used before multiple bit reads
    /// Returns a buffer containing the next k bits (for lookahead)
    /// 
    /// Note: Kept for compatibility with C++ BitDecoder::EnsureBits
    /// and for potential future optimizations
    #[allow(dead_code)]
    pub fn ensure_bits(&self, k: usize) -> u32 {
        debug_assert!(k <= 24, "Cannot ensure more than 24 bits");
        debug_assert!(k <= self.available_bits(), "Not enough bits available");

        let mut buf = 0u32;
        for i in 0..k {
            buf |= (self.peek_bit(i) as u32) << i;
        }
        buf
    }

    /// Get bits from the buffer
    pub fn get_bits(&mut self, nbits: u8) -> Result<u32, &'static str> {
        if nbits > 32 {
            return Err("Cannot decode more than 32 bits");
        }

        if self.bit_offset + nbits as usize > self.available_bits() {
            return Err("Not enough bits available");
        }

        let mut value = 0u32;
        for bit in 0..nbits {
            value |= (self.get_bit() as u32) << bit;
        }

        Ok(value)
    }

    /// Get a single bit from the buffer
    fn get_bit(&mut self) -> i32 {
        let off = self.bit_offset;
        let byte_offset = off >> 3;
        let bit_shift = (off & 0x7) as u8;

        unsafe {
            if self.bit_buffer.add(byte_offset) < self.bit_buffer_end {
                let bit = (*self.bit_buffer.add(byte_offset) >> bit_shift) & 1;
                self.bit_offset = off + 1;
                bit as i32
            } else {
                0
            }
        }
    }

    /// Peek at a bit without consuming it
    /// Used by ensure_bits() for lookahead operations
    fn peek_bit(&self, offset: usize) -> i32 {
        let off = self.bit_offset + offset;
        let byte_offset = off >> 3;
        let bit_shift = (off & 0x7) as u8;

        unsafe {
            if self.bit_buffer.add(byte_offset) < self.bit_buffer_end {
                ((*self.bit_buffer.add(byte_offset) >> bit_shift) & 1) as i32
            } else {
                0
            }
        }
    }
}

impl Default for DecoderBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for BitDecoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoder_buffer_creation() {
        let buf = DecoderBuffer::new();
        assert_eq!(buf.position(), 0);
        assert_eq!(buf.data_size(), 0);
        assert!(!buf.bit_decoder_active());
    }

    #[test]
    fn test_byte_aligned_decoding() {
        let data = [0x78, 0x56, 0x34, 0x12]; // Little endian 0x12345678
        let mut buf = DecoderBuffer::new();

        unsafe {
            buf.init(data.as_ptr(), data.len());
        }

        // Decode the integer
        let value: u32 = unsafe { buf.decode_type().unwrap() };
        assert_eq!(value, 0x12345678);
        assert_eq!(buf.position(), 4);
        assert_eq!(buf.remaining_size(), 0);
    }

    #[test]
    fn test_byte_decoding() {
        let data = b"Hello";
        let mut buf = DecoderBuffer::new();

        unsafe {
            buf.init(data.as_ptr(), data.len());
        }

        let mut out_data = [0u8; 3];
        buf.decode(&mut out_data).unwrap();

        assert_eq!(&out_data, b"Hel");
        assert_eq!(buf.position(), 3);
        assert_eq!(buf.remaining_size(), 2);
    }

    #[test]
    fn test_peek_operations() {
        let data = [0x78, 0x56, 0x34, 0x12]; // Little endian 0x12345678
        let mut buf = DecoderBuffer::new();

        unsafe {
            buf.init(data.as_ptr(), data.len());
        }

        // Peek at the value
        let value: u32 = unsafe { buf.peek_type().unwrap() };
        assert_eq!(value, 0x12345678);
        assert_eq!(buf.position(), 0); // Position shouldn't change

        // Advance and peek again - only peek at what's available
        buf.advance(1).unwrap();
        // Now we only have 3 bytes left, so let's peek at them as a u16 instead
        let value: u16 = unsafe { buf.peek_type().unwrap() };
        // Should be 0x3456 (little endian reading of [0x56, 0x34])
        assert_eq!(value, 0x3456);
    }

    #[test]
    fn test_advance_and_positioning() {
        let data = [1, 2, 3, 4, 5];
        let mut buf = DecoderBuffer::new();

        unsafe {
            buf.init(data.as_ptr(), data.len());
        }

        assert_eq!(buf.position(), 0);

        buf.advance(2).unwrap();
        assert_eq!(buf.position(), 2);
        assert_eq!(buf.remaining_size(), 3);

        buf.start_decoding_from(1).unwrap();
        assert_eq!(buf.position(), 1);
    }

    #[test]
    fn test_error_conditions() {
        let data = [1, 2, 3];
        let mut buf = DecoderBuffer::new();

        unsafe {
            buf.init(data.as_ptr(), data.len());
        }

        // Try to read past end
        let mut out_data = [0u8; 5];
        assert!(buf.decode(&mut out_data).is_err());

        // Try to advance past end
        assert!(buf.advance(5).is_err());

        // Try to set position beyond buffer
        assert!(buf.start_decoding_from(5).is_err());

        // Try bit operations without starting bit decoding
        assert!(buf.decode_least_significant_bits_32(8).is_err());
    }

    #[test]
    fn test_bitstream_version() {
        let mut buf = DecoderBuffer::new();
        assert_eq!(buf.bitstream_version(), 0);

        buf.set_bitstream_version(42);
        assert_eq!(buf.bitstream_version(), 42);

        let data = [1, 2, 3];
        unsafe {
            buf.init_with_version(data.as_ptr(), data.len(), 123);
        }
        assert_eq!(buf.bitstream_version(), 123);
    }
}