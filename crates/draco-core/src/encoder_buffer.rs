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

use std::slice;

/// Encoder buffer for Draco serialization with zero-copy optimizations
/// Supports both byte-aligned and bit-level encoding
#[derive(Debug, Clone)]
pub struct EncoderBuffer {
    /// Main buffer data
    buffer: Vec<u8>,

    /// Bit encoder state (active when encoding bits)
    bit_encoder: Option<BitEncoder>,

    /// Number of bytes reserved for bit encoding
    bit_encoder_reserved_bytes: usize,

    /// Whether to encode bit sequence size
    encode_bit_sequence_size: bool,
}

/// Bit encoder for variable-length bit sequences
#[derive(Debug, Clone)]
pub struct BitEncoder {
    /// Current bit position within the buffer
    bit_offset: usize,

    /// Starting byte position for bit encoding
    start_byte_offset: usize,
}

impl EncoderBuffer {
    /// Create a new empty encoder buffer
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            bit_encoder: None,
            bit_encoder_reserved_bytes: 0,
            encode_bit_sequence_size: false,
        }
    }

    /// Create a buffer with initial capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            bit_encoder: None,
            bit_encoder_reserved_bytes: 0,
            encode_bit_sequence_size: false,
        }
    }

    /// Clear all data from the buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.bit_encoder = None;
        self.bit_encoder_reserved_bytes = 0;
        self.encode_bit_sequence_size = false;
    }

    /// Resize buffer to specified size
    pub fn resize(&mut self, nbytes: usize) {
        self.buffer.resize(nbytes, 0);
    }

    /// Get the current buffer data
    pub fn data(&self) -> &[u8] {
        &self.buffer
    }

    /// Get the current buffer size
    pub fn size(&self) -> usize {
        self.buffer.len()
    }

    /// Get the current buffer capacity
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    /// Get mutable reference to buffer (for C API compatibility)
    pub fn buffer_mut(&mut self) -> &mut Vec<u8> {
        &mut self.buffer
    }

    /// Check if bit encoder is currently active
    pub fn bit_encoder_active(&self) -> bool {
        self.bit_encoder.is_some()
    }

    /// Start encoding a bit sequence
    ///
    /// # Arguments
    /// * `required_bits` - Maximum number of bits needed
    /// * `encode_size` - Whether to encode the size of the bit sequence
    pub fn start_bit_encoding(&mut self, required_bits: usize, encode_size: bool) -> Result<(), &'static str> {
        if self.bit_encoder_active() {
            return Err("Bit encoder already active");
        }

        let required_bytes = (required_bits + 7) / 8;

        // Reserve space for size encoding if requested
        if encode_size {
            // Encode size as 32-bit integer (little endian)
            self.buffer.extend_from_slice(&0u32.to_le_bytes());
        }

        // Reserve space for bit data and pad with zeros
        let start_pos = self.buffer.len();
        self.buffer.resize(start_pos + required_bytes, 0);

        self.bit_encoder = Some(BitEncoder {
            bit_offset: 0,
            start_byte_offset: start_pos,
        });
        self.bit_encoder_reserved_bytes = required_bytes;
        self.encode_bit_sequence_size = encode_size;

        Ok(())
    }

    /// End bit encoding and finalize the buffer
    pub fn end_bit_encoding(&mut self) {
        if let Some(encoder) = &self.bit_encoder {
            // Calculate actual bytes used
            let actual_bytes = (encoder.bit_offset + 7) / 8;

            // Resize buffer to actual size
            let end_pos = encoder.start_byte_offset + actual_bytes;
            self.buffer.truncate(end_pos);

            // If we need to encode size, update it
            if self.encode_bit_sequence_size {
                let size_offset = encoder.start_byte_offset - 4;
                let actual_bits = encoder.bit_offset as u32;
                self.buffer[size_offset..size_offset + 4].copy_from_slice(&actual_bits.to_le_bytes());
            }
        }

        self.bit_encoder = None;
        self.bit_encoder_reserved_bytes = 0;
        self.encode_bit_sequence_size = false;
    }

    /// Encode least significant bits of a value
    ///
    /// # Arguments
    /// * `nbits` - Number of bits to encode (0-32)
    /// * `value` - Value to encode
    pub fn encode_least_significant_bits_32(&mut self, nbits: u8, value: u32) -> Result<(), &'static str> {
        if nbits > 32 {
            return Err("Cannot encode more than 32 bits");
        }

        if let Some(ref mut encoder) = self.bit_encoder {
            if encoder.bit_offset + nbits as usize > self.bit_encoder_reserved_bytes * 8 {
                return Err("Bit sequence exceeds reserved space");
            }

            let buffer_end = encoder.start_byte_offset + self.bit_encoder_reserved_bytes;
            if buffer_end > self.buffer.len() {
                return Err("Buffer too small for bit encoding");
            }

            let buffer_slice = &mut self.buffer[encoder.start_byte_offset..buffer_end];
            encoder.put_bits(value, nbits, buffer_slice);
            Ok(())
        } else {
            Err("Bit encoder not active")
        }
    }

    /// Encode arbitrary data type (byte-aligned)
    ///
    /// # Safety
    /// This function uses unsafe code to convert any type to bytes.
    /// Ensure the type is POD (plain old data) and doesn't contain references.
    pub unsafe fn encode_type<T>(&mut self, data: &T) -> Result<(), &'static str> {
        if self.bit_encoder_active() {
            return Err("Cannot encode byte-aligned data during bit encoding");
        }

        let bytes = slice::from_raw_parts(data as *const T as *const u8, std::mem::size_of::<T>());
        self.buffer.extend_from_slice(bytes);
        Ok(())
    }

    /// Encode raw bytes (byte-aligned)
    pub fn encode(&mut self, data: &[u8]) -> Result<(), &'static str> {
        if self.bit_encoder_active() {
            return Err("Cannot encode byte-aligned data during bit encoding");
        }

        self.buffer.extend_from_slice(data);
        Ok(())
    }
}

impl BitEncoder {
    /// Write up to 32 bits to the buffer
    pub fn put_bits(&mut self, data: u32, nbits: u8, buffer: &mut [u8]) {
        debug_assert!(nbits <= 32, "Cannot write more than 32 bits");

        for bit in 0..nbits {
            self.put_bit((data >> bit) & 1, buffer);
        }
    }

    /// Get the current bit offset
    pub fn bit_offset(&self) -> usize {
        self.bit_offset
    }

    /// Get number of bits required to store the given number
    pub fn bits_required(mut x: u32) -> u32 {
        if x == 0 {
            return 0;
        }

        let mut bits = 0;
        while x > 0 {
            bits += 1;
            x >>= 1;
        }
        bits
    }

    /// Write a single bit to the buffer
    ///
    /// # Arguments
    /// * `value` - Bit value (0 or 1)
    fn put_bit(&mut self, value: u32, buffer: &mut [u8]) {
        let bit_offset = self.bit_offset;
        let byte_offset = bit_offset / 8;
        let bit_shift = (bit_offset % 8) as u8;

        if byte_offset >= buffer.len() {
            panic!("Buffer overflow during bit encoding");
        }

        // Clear the target bit and set it to the new value
        buffer[byte_offset] &= !(1 << bit_shift);
        buffer[byte_offset] |= (value as u8) << bit_shift;

        self.bit_offset += 1;
    }
}

impl Default for EncoderBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_buffer_creation() {
        let buf = EncoderBuffer::new();
        assert_eq!(buf.size(), 0);
        assert!(!buf.bit_encoder_active());
    }

    #[test]
    fn test_encoder_buffer_with_capacity() {
        let buf = EncoderBuffer::with_capacity(1024);
        assert_eq!(buf.size(), 0);
        assert!(buf.capacity() >= 1024);
    }

    #[test]
    fn test_byte_aligned_encoding() {
        let mut buf = EncoderBuffer::new();

        // Encode a simple integer
        let value: u32 = 0x12345678;
        unsafe {
            buf.encode_type(&value).unwrap();
        }

        assert_eq!(buf.size(), 4);
        let encoded = buf.data();
        assert_eq!(encoded, &[0x78, 0x56, 0x34, 0x12]); // Little endian
    }

    #[test]
    fn test_raw_byte_encoding() {
        let mut buf = EncoderBuffer::new();

        let data = b"Hello, World!";
        buf.encode(data).unwrap();

        assert_eq!(buf.size(), 13);
        assert_eq!(buf.data(), data);
    }

    #[test]
    fn test_bit_encoding() {
        let mut buf = EncoderBuffer::new();

        buf.start_bit_encoding(16, false).unwrap();
        buf.encode_least_significant_bits_32(4, 0b1010).unwrap();
        buf.encode_least_significant_bits_32(4, 0b1100).unwrap();
        buf.encode_least_significant_bits_32(8, 0xFF).unwrap();
        buf.end_bit_encoding();

        // Should have 2 bytes for the encoded bits
        assert_eq!(buf.size(), 2);
        // The first byte should contain 0b11001010 (LSB first)
        assert_eq!(buf.data()[0], 0b11001010);
        // The second byte should contain 0b11111111
        assert_eq!(buf.data()[1], 0b11111111);
    }

    #[test]
    fn test_bit_encoding_with_size() {
        let mut buf = EncoderBuffer::new();

        buf.start_bit_encoding(8, true).unwrap();
        buf.encode_least_significant_bits_32(4, 0b1010).unwrap();
        buf.encode_least_significant_bits_32(4, 0b1100).unwrap();
        buf.end_bit_encoding();

        // Should have 4 bytes for size + 1 byte for actual data
        assert_eq!(buf.size(), 5);
        // First 4 bytes should contain the bit count (8) in little endian
        assert_eq!(&buf.data()[..4], &[8, 0, 0, 0]);
        // The next byte should contain 0b11001010 (LSB first)
        assert_eq!(buf.data()[4], 0b11001010);
    }

    #[test]
    fn test_simple_bit_encoding() {
        let mut buf = EncoderBuffer::new();

        // Test basic bit encoding without size
        buf.start_bit_encoding(8, false).unwrap();
        buf.encode_least_significant_bits_32(1, 1).unwrap();
        buf.encode_least_significant_bits_32(1, 0).unwrap();
        buf.encode_least_significant_bits_32(1, 1).unwrap();
        buf.encode_least_significant_bits_32(1, 1).unwrap();
        buf.end_bit_encoding();

        assert_eq!(buf.size(), 1);
        // Should have 0b1101 (LSB first: 0b1011)
        assert_eq!(buf.data()[0], 0b00001101);
    }

    #[test]
    fn test_clear_and_resize() {
        let mut buf = EncoderBuffer::new();

        buf.encode(b"test").unwrap();
        assert_eq!(buf.size(), 4);

        buf.clear();
        assert_eq!(buf.size(), 0);

        buf.resize(100);
        assert_eq!(buf.size(), 100);
        assert_eq!(buf.data(), &[0; 100]);
    }

    #[test]
    fn test_error_conditions() {
        let mut buf = EncoderBuffer::new();

        // Can't encode bits without starting bit encoding
        assert!(buf.encode_least_significant_bits_32(8, 0xFF).is_err());

        // Can't encode byte data during bit encoding
        buf.start_bit_encoding(8, false).unwrap();
        assert!(buf.encode(b"test").is_err());
        assert!(unsafe { buf.encode_type(&42u32) }.is_err());
    }

    #[test]
    fn test_bits_required() {
        assert_eq!(BitEncoder::bits_required(0), 0);
        assert_eq!(BitEncoder::bits_required(1), 1);
        assert_eq!(BitEncoder::bits_required(2), 2);
        assert_eq!(BitEncoder::bits_required(3), 2);
        assert_eq!(BitEncoder::bits_required(4), 3);
        assert_eq!(BitEncoder::bits_required(7), 3);
        assert_eq!(BitEncoder::bits_required(8), 4);
        assert_eq!(BitEncoder::bits_required(255), 8);
        assert_eq!(BitEncoder::bits_required(256), 9);
    }
}