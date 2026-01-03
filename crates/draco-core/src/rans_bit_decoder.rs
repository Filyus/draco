use crate::ans::AnsDecoder;
use crate::decoder_buffer::DecoderBuffer;

pub struct RAnsBitDecoder<'a> {
    ans_decoder: Option<AnsDecoder<'a>>,
    prob_zero: u8,
}

impl<'a> Default for RAnsBitDecoder<'a> {
    fn default() -> Self {
        Self {
            ans_decoder: None,
            prob_zero: 0,
        }
    }
}

impl<'a> RAnsBitDecoder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_decoding(&mut self, source_buffer: &mut DecoderBuffer<'a>) -> bool {
        self.clear();
        
        // Read zero_prob
        if let Ok(prob) = source_buffer.decode::<u8>() {
            self.prob_zero = prob;
        } else {
            return false;
        }

        // Read size_in_bytes.
        // The Rust encoder always writes varint (matching C++ >= 2.2 behavior).
        // We read varint unconditionally since:
        // 1. Our encoder writes varint
        // 2. C++ files with version >= 2.2 write varint
        // 3. Version 0.0 means unit test without header (use our encoder format)
        // Note: C++ has backward compat for pre-2.2 files that wrote u32, but
        // Rust doesn't need to support those legacy formats.
        let size: u32 = match source_buffer.decode_varint() {
            Ok(v) => v as u32,
            Err(_) => return false,
        };

        if let Ok(slice) = source_buffer.decode_slice(size as usize) {
            let mut decoder = AnsDecoder::new(slice);
            if decoder.read_init(crate::ans::ANS_L_BASE) {
                self.ans_decoder = Some(decoder);
                return true;
            }
        }
        
        return false; 
    }

    pub fn decode_next_bit(&mut self) -> bool {
        if let Some(decoder) = &mut self.ans_decoder {
            decoder.rabs_desc_read(self.prob_zero)
        } else {
            false
        }
    }

    pub fn decode_least_significant_bits32(&mut self, nbits: i32, value: &mut u32) {
        // Match Draco C++: accumulate bits MSB-first.
        *value = 0;
        for _ in 0..nbits {
            let bit = self.decode_next_bit();
            *value = (*value << 1) + (bit as u32);
        }
    }

    pub fn end_decoding(&mut self) {
        self.ans_decoder = None;
    }

    fn clear(&mut self) {
        self.ans_decoder = None;
        self.prob_zero = 0;
    }
}
