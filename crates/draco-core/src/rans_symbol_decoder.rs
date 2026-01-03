use crate::ans::AnsDecoder;
use crate::decoder_buffer::DecoderBuffer;
use crate::rans_symbol_coding::RAnsSymbol;

pub struct RAnsSymbolDecoder<'a, const RANS_PRECISION_BITS: u32> {
    pub ans: AnsDecoder<'a>,
    probability_table: Vec<RAnsSymbol>,
    lut: Vec<u32>,
    num_symbols: usize,
}

impl<'a, const RANS_PRECISION_BITS: u32> RAnsSymbolDecoder<'a, RANS_PRECISION_BITS> {
    const RANS_PRECISION: u32 = 1 << RANS_PRECISION_BITS;
    const L_RANS_BASE: u32 = Self::RANS_PRECISION * 4;

    pub fn new() -> Self {
        Self {
            ans: AnsDecoder::new(&[]),
            probability_table: Vec::new(),
            lut: Vec::new(),
            num_symbols: 0,
        }
    }

    pub fn create(&mut self, buffer: &mut DecoderBuffer) -> bool {
        if !self.decode_table(buffer) {
            return false;
        }
        true
    }

    fn decode_table(&mut self, buffer: &mut DecoderBuffer) -> bool {
        let _start_pos = buffer.position();
        let bitstream_version = ((buffer.version_major() as u16) << 8) | (buffer.version_minor() as u16);
        let num_symbols = if bitstream_version < 0x0200 {
            match buffer.decode_u8() {
                Ok(v) => v as usize,
                Err(_) => return false,
            }
        } else {
            match buffer.decode_varint() {
                Ok(v) => v as usize,
                Err(_) => return false,
            }
        };
        self.num_symbols = num_symbols;
        if num_symbols == 0 {
            return true;
        }

        self.probability_table.resize(num_symbols, RAnsSymbol::default());

        // NOTE: C++ only early-returns for num_symbols == 0.
        // For num_symbols == 1, it still reads the probability table byte.
        // We must do the same to stay in sync with the buffer!
        
        let mut i = 0;
        while i < num_symbols {
            let b = match buffer.decode_u8() {
                Ok(v) => v,
                Err(_) => return false,
            };
            
            let mode = b & 3;
            if mode == 3 {
                // Zero frequency offset
                let offset = (b >> 2) as usize;
                for j in 0..=offset {
                    if i + j >= num_symbols {
                        return false;
                    }
                    self.probability_table[i + j].prob = 0;
                }
                i += offset;
            } else {
                let num_extra_bytes = mode as usize;
                let mut prob = (b >> 2) as u32;
                for b_idx in 0..num_extra_bytes {
                    let extra = match buffer.decode_u8() {
                        Ok(v) => v,
                        Err(_) => return false,
                    };
                    prob |= (extra as u32) << (8 * (b_idx + 1) - 2);
                }
                self.probability_table[i].prob = prob;
            }
            i += 1;
        }
        
        // Compute cumulative probabilities and LUT
        self.lut.resize(Self::RANS_PRECISION as usize, 0);
        let mut cum_prob: u32 = 0;
        for i in 0..num_symbols {
            let prob = self.probability_table[i].prob;
            self.probability_table[i].cum_prob = cum_prob;
            // Bounds check: ensure we don't write past the LUT
            let end_idx = cum_prob.saturating_add(prob);
            if end_idx > Self::RANS_PRECISION {
                // Malformed probability table - probabilities exceed precision
                return false;
            }
            for j in 0..prob {
                self.lut[(cum_prob + j) as usize] = i as u32;
            }
            cum_prob = end_idx;
        }
        
        if cum_prob != Self::RANS_PRECISION {
            return false;
        }
        true
    }

    pub fn start_decoding(&mut self, buffer: &mut DecoderBuffer<'a>) -> bool {
        // Draco advances the buffer past the encoded rANS data regardless of the
        // number of symbols (the encoded size prefix is always present).
        // 
        // Note: The size is always encoded as varint, even in pre-v2.0 bitstreams.
        // Only the num_symbols count uses version-specific encoding (u8 vs varint).
        let bytes_to_read = match buffer.decode_varint() {
            Ok(v) => v as usize,
            Err(_) => return false,
        };
        if self.num_symbols <= 1 {
            // Still need to advance the buffer past the encoded bytes.
            buffer.advance(bytes_to_read);
            return true;
        }
        let data = buffer.remaining_data();
        if data.len() < bytes_to_read {
            return false;
        }
        
        let rans_data = &data[..bytes_to_read];
        self.ans = AnsDecoder::new(rans_data);
        if !self.ans.read_init(Self::L_RANS_BASE) {
            return false;
        }
        
        buffer.advance(bytes_to_read);
        true
    }

    pub fn decode_symbol(&mut self) -> u32 {
        if self.num_symbols <= 1 {
            return 0;
        }
        // Match Draco C++ (ans.h) rans_read(): normalize first, then use
        // division/modulo by rans_precision (power of two).
        self.ans.read_normalize();
        let quo = self.ans.state / Self::RANS_PRECISION;
        let rem = self.ans.state % Self::RANS_PRECISION;
        let symbol_id = self.lut[rem as usize];

        let sym = &self.probability_table[symbol_id as usize];
        self.ans.state = quo * sym.prob + rem - sym.cum_prob;
        symbol_id
    }
}
