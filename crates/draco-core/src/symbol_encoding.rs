use crate::decoder_buffer::DecoderBuffer;
use crate::encoder_buffer::EncoderBuffer;
use crate::rans_symbol_coding::{
    approximate_rans_frequency_table_bits, compute_rans_precision_from_unique_symbols_bit_length,
};
use crate::rans_symbol_decoder::RAnsSymbolDecoder;
use crate::rans_symbol_encoder::RAnsSymbolEncoder;

pub struct SymbolEncodingOptions {
    pub compression_level: i32,
}

impl Default for SymbolEncodingOptions {
    fn default() -> Self {
        Self {
            compression_level: 7,
        }
    }
}

pub fn encode_symbols(
    symbols: &[u32],
    num_components: usize,
    _options: &SymbolEncodingOptions,
    target_buffer: &mut EncoderBuffer,
) -> bool {
    if symbols.is_empty() {
        return true;
    }

    // Compute bit lengths
    let mut bit_lengths = Vec::with_capacity(symbols.len());
    let mut max_value = 0;
    
    for chunk in symbols.chunks(num_components) {
        let mut max_component_value = chunk[0];
        for &val in &chunk[1..] {
            if val > max_component_value {
                max_component_value = val;
            }
        }
        
        // C++ uses: value_msb_pos = MostSignificantBit(max_component_value);
        //           bit_lengths.push(value_msb_pos + 1);
        // MostSignificantBit returns 0-indexed position, so +1 gives bit count.
        // For max_component_value == 0, C++ uses value_msb_pos = 0, so bit_length = 1.
        let bit_length = if max_component_value > 0 {
            32 - max_component_value.leading_zeros()
        } else {
            1 // Minimum 1 bit, matching C++ behavior
        };
        if max_component_value > max_value {
            max_value = max_component_value;
        }
        bit_lengths.push(bit_length);
    }

    // Estimate bits for tagged scheme
    let tagged_bits = compute_tagged_scheme_bits(symbols, num_components, &bit_lengths, max_value);
    
    // Estimate bits for raw scheme
    let raw_bits = compute_raw_scheme_bits(symbols, max_value);
    
    let max_value_bit_length = if max_value == 0 { 0 } else { 32 - max_value.leading_zeros() };
    const K_MAX_RAW_ENCODING_BIT_LENGTH: u32 = 18;

    if tagged_bits < raw_bits || max_value_bit_length > K_MAX_RAW_ENCODING_BIT_LENGTH {
        // Draco bitstream scheme ids (see C++ SymbolCodingMethod):
        //   0 = TAGGED
        //   1 = RAW
        target_buffer.encode_u8(0); // TAGGED
        encode_tagged_symbols(symbols, num_components, &bit_lengths, target_buffer)
    } else {
        target_buffer.encode_u8(1); // RAW
        encode_raw_symbols(symbols, max_value, target_buffer)
    }
}

pub fn estimate_bits(symbols: &[u32], num_components: usize) -> u64 {
    if symbols.is_empty() {
        return 0;
    }

    // Compute bit lengths
    let mut bit_lengths = Vec::with_capacity(symbols.len());
    let mut max_value = 0;
    
    for chunk in symbols.chunks(num_components) {
        let mut max_component_value = chunk[0];
        for &val in &chunk[1..] {
            if val > max_component_value {
                max_component_value = val;
            }
        }
        
        // C++ uses: value_msb_pos = MostSignificantBit(max_component_value);
        //           bit_lengths.push(value_msb_pos + 1);
        // For max_component_value == 0, bit_length = 1.
        let bit_length = if max_component_value > 0 {
            32 - max_component_value.leading_zeros()
        } else {
            1 // Minimum 1 bit, matching C++ behavior
        };
        if max_component_value > max_value {
            max_value = max_component_value;
        }
        bit_lengths.push(bit_length);
    }

    let tagged_bits = compute_tagged_scheme_bits(symbols, num_components, &bit_lengths, max_value);
    let raw_bits = compute_raw_scheme_bits(symbols, max_value);
    
    std::cmp::min(tagged_bits, raw_bits)
}

fn compute_raw_scheme_bits(symbols: &[u32], max_value: u32) -> u64 {
    // Count frequencies
    let num_unique_symbols = (max_value + 1) as usize;
    let mut frequencies = vec![0u64; num_unique_symbols];
    for &sym in symbols {
        frequencies[sym as usize] += 1;
    }
    
    let mut total_freq = 0;
    let mut num_present_symbols: u32 = 0;
    for &freq in &frequencies {
        if freq > 0 {
            total_freq += freq;
            num_present_symbols += 1;
        }
    }
    
    if total_freq == 0 {
        return 0;
    }
    
    // Shannon entropy
    let mut entropy_bits = 0.0;
    let total_freq_f = total_freq as f64;
    for &freq in &frequencies {
        if freq > 0 {
            let p = freq as f64 / total_freq_f;
            entropy_bits += -p.log2() * freq as f64;
        }
    }
    
    let table_bits = approximate_rans_frequency_table_bits(max_value, num_present_symbols);
    
    (entropy_bits.ceil() as u64) + table_bits
}

fn compute_tagged_scheme_bits(
    _symbols: &[u32],
    num_components: usize,
    bit_lengths: &[u32],
    _max_value: u32,
) -> u64 {
    // 1. Bits for values (raw bits)
    let mut value_bits = 0;
    for (_i, &len) in bit_lengths.iter().enumerate() {
        value_bits += len as u64 * num_components as u64;
    }
    
    // 2. Bits for tags (RAns)
    // Count tag frequencies
    let mut tag_frequencies = vec![0u64; 33];
    for &len in bit_lengths {
        tag_frequencies[len as usize] += 1;
    }
    
    let mut total_tags = 0;
    let mut num_present_tags: u32 = 0;
    for &freq in &tag_frequencies {
        if freq > 0 {
            total_tags += freq;
            num_present_tags += 1;
        }
    }
    
    if total_tags == 0 {
        return value_bits;
    }
    
    let mut tag_entropy_bits = 0.0;
    let total_tags_f = total_tags as f64;
    for &freq in &tag_frequencies {
        if freq > 0 {
            let p = freq as f64 / total_tags_f;
            tag_entropy_bits += -p.log2() * freq as f64;
        }
    }
    
    let table_bits = approximate_rans_frequency_table_bits(32, num_present_tags);
    
    value_bits + (tag_entropy_bits.ceil() as u64) + table_bits
}

pub fn encode_raw_symbols(symbols: &[u32], max_value: u32, target_buffer: &mut EncoderBuffer) -> bool {
    // num_values is known by decoder

    // Count frequencies
    let mut frequencies = vec![0u64; (max_value + 1) as usize];
    for &s in symbols {
        frequencies[s as usize] += 1;
    }
    
    let mut num_unique_symbols: u32 = 0;
    for &f in &frequencies {
        if f > 0 {
            num_unique_symbols += 1;
        }
    }
    
    let mut unique_symbols_bit_length: u32 = if num_unique_symbols > 0 {
        32 - num_unique_symbols.leading_zeros()
    } else {
        0
    };
    
    // Compression level adjustment (default 7)
    let compression_level = 7;
    if compression_level < 4 {
        unique_symbols_bit_length = unique_symbols_bit_length.saturating_sub(2);
    } else if compression_level < 6 {
        unique_symbols_bit_length = unique_symbols_bit_length.saturating_sub(1);
    } else if compression_level > 9 {
        unique_symbols_bit_length += 2;
    } else if compression_level > 7 {
        unique_symbols_bit_length += 1;
    }
    
    unique_symbols_bit_length = unique_symbols_bit_length.max(1).min(18);
    
    target_buffer.encode_u8(unique_symbols_bit_length as u8);
    
    let rans_precision_bits = compute_rans_precision_from_unique_symbols_bit_length(unique_symbols_bit_length);
    
    match rans_precision_bits {
        12 => encode_raw_symbols_internal::<12>(symbols, &frequencies, target_buffer),
        13 => encode_raw_symbols_internal::<13>(symbols, &frequencies, target_buffer),
        14 => encode_raw_symbols_internal::<14>(symbols, &frequencies, target_buffer),
        15 => encode_raw_symbols_internal::<15>(symbols, &frequencies, target_buffer),
        16 => encode_raw_symbols_internal::<16>(symbols, &frequencies, target_buffer),
        17 => encode_raw_symbols_internal::<17>(symbols, &frequencies, target_buffer),
        18 => encode_raw_symbols_internal::<18>(symbols, &frequencies, target_buffer),
        19 => encode_raw_symbols_internal::<19>(symbols, &frequencies, target_buffer),
        20 => encode_raw_symbols_internal::<20>(symbols, &frequencies, target_buffer),
        _ => false,
    }
}

fn encode_raw_symbols_internal<const RANS_PRECISION_BITS: u32>(
    symbols: &[u32],
    frequencies: &[u64],
    target_buffer: &mut EncoderBuffer
) -> bool {
    let mut encoder = RAnsSymbolEncoder::<RANS_PRECISION_BITS>::new();
    encoder.create(frequencies, frequencies.len(), target_buffer);
    encoder.start_encoding(target_buffer);
    
    // Reverse encoding
    for &sym in symbols.iter().rev() {
        encoder.encode_symbol(sym);
    }
    
    encoder.end_encoding(target_buffer);
    true
}

/*
pub fn encode_raw_symbols_no_scheme(symbols: &[u32], max_value: u32, target_buffer: &mut EncoderBuffer) -> bool {
    // ...
}
*/


#[allow(dead_code)]
fn encode_raw_symbols_typed<const PRECISION_BITS: u32>(
    symbols: &[u32],
    frequencies: &[u64],
    num_unique_symbols: usize,
    target_buffer: &mut EncoderBuffer,
) -> bool {
    let mut encoder = RAnsSymbolEncoder::<PRECISION_BITS>::new();
    if !encoder.create(frequencies, num_unique_symbols, target_buffer) {
        return false;
    }
    
    encoder.start_encoding(target_buffer);
    for &sym in symbols.iter().rev() {
        encoder.encode_symbol(sym);
    }
    encoder.end_encoding(target_buffer);
    true
}

fn encode_tagged_symbols(
    symbols: &[u32],
    num_components: usize,
    bit_lengths: &[u32],
    target_buffer: &mut EncoderBuffer,
) -> bool {
    // Scheme: Tagged is already written by caller

    // Encode bit lengths using RAns
    // Count frequencies of bit lengths (0..32)
    let mut frequencies = vec![0u64; 33];
    for &len in bit_lengths {
        frequencies[len as usize] += 1;
    }
    
    // Draco uses unique_symbols_bit_length=5 for tagged bit-length tags,
    // which corresponds to rANS precision bits = 12.
    let mut tag_encoder = RAnsSymbolEncoder::<12>::new();
    if !tag_encoder.create(&frequencies, 33, target_buffer) {
        return false;
    }
    
    // Create a separate bit buffer for raw values (C++ value_buffer)
    let mut value_buffer = EncoderBuffer::new();
    let value_bits = 32 * (symbols.len()); // safe upper bound
    value_buffer.start_bit_encoding(value_bits, false);

    tag_encoder.start_encoding(target_buffer);
    
    // 1. Encode bits in FORWARD order (because our BitEncoder is FIFO).
    for (i, &len) in bit_lengths.iter().enumerate() {
        let val_idx = i * num_components;
        for j in 0..num_components {
            let val = symbols[val_idx + j];
            value_buffer.encode_least_significant_bits32(len, val);
        }
    }
    
    // 2. Encode tags in REVERSE order (because ANS is LIFO).
    for &len in bit_lengths.iter().rev() {
        tag_encoder.encode_symbol(len);
    }
    
    tag_encoder.end_encoding(target_buffer);
    value_buffer.end_bit_encoding();
    target_buffer.encode_data(value_buffer.data());
    true
}

pub fn decode_symbols(
    num_values: usize,
    num_components: usize,
    _options: &SymbolEncodingOptions,
    in_buffer: &mut DecoderBuffer,
    symbols: &mut [u32],
) -> bool {
    if num_values == 0 {
        return true;
    }

    let scheme = match in_buffer.decode_u8() {
        Ok(v) => v,
        Err(_) => return false,
    };

    // Support both the older internal ids (0/1) and the Draco ids (2/3).
    // Draco uses: 2 = TAGGED, 3 = RAW.
    match scheme {
        0 | 2 => decode_tagged_symbols(num_values, num_components, in_buffer, symbols),
        1 | 3 => decode_raw_symbols(num_values, in_buffer, symbols),
        _ => false,
    }
}

pub fn decode_raw_symbols(num_values: usize, in_buffer: &mut DecoderBuffer, symbols: &mut [u32]) -> bool {
    // Read serialized symbol-bit-length header (written by encoder)
    let symbols_bit_length = match in_buffer.decode_u8() {
        Ok(v) => v as u32,
        Err(_) => return false,
    };
    if symbols_bit_length == 0 {
        for i in 0..num_values {
            symbols[i] = 0;
        }
        return true;
    }
    let unique_symbols_bit_length = symbols_bit_length;
    let precision_bits =
        compute_rans_precision_from_unique_symbols_bit_length(unique_symbols_bit_length);

    match precision_bits {
        12 => decode_raw_symbols_typed::<12>(num_values, in_buffer, symbols),
        13 => decode_raw_symbols_typed::<13>(num_values, in_buffer, symbols),
        14 => decode_raw_symbols_typed::<14>(num_values, in_buffer, symbols),
        15 => decode_raw_symbols_typed::<15>(num_values, in_buffer, symbols),
        16 => decode_raw_symbols_typed::<16>(num_values, in_buffer, symbols),
        17 => decode_raw_symbols_typed::<17>(num_values, in_buffer, symbols),
        18 => decode_raw_symbols_typed::<18>(num_values, in_buffer, symbols),
        19 => decode_raw_symbols_typed::<19>(num_values, in_buffer, symbols),
        20 => decode_raw_symbols_typed::<20>(num_values, in_buffer, symbols),
        _ => false,
    }
}

fn decode_raw_symbols_typed<const PRECISION_BITS: u32>(
    num_values: usize,
    in_buffer: &mut DecoderBuffer,
    symbols: &mut [u32],
) -> bool {
    let mut decoder = RAnsSymbolDecoder::<PRECISION_BITS>::new();
    if !decoder.create(in_buffer) {
        return false;
    }
    if !decoder.start_decoding(in_buffer) {
        return false;
    }
    for i in 0..num_values {
        symbols[i] = decoder.decode_symbol();
    }
    true
}

fn decode_tagged_symbols(
    num_values: usize,
    num_components: usize,
    in_buffer: &mut DecoderBuffer,
    symbols: &mut [u32],
) -> bool {
    // C++ uses RAnsSymbolDecoder<5> where 5 is unique_symbols_bit_length.
    // This maps to precision_bits = 12 via ComputeRAnsPrecisionFromUniqueSymbolsBitLength.
    let mut tag_decoder = RAnsSymbolDecoder::<12>::new();

    if !tag_decoder.create(in_buffer) {
        return false;
    }
    if !tag_decoder.start_decoding(in_buffer) {
        return false;
    }

    // Start bit-decoding for raw values (value_buffer)
    if in_buffer.start_bit_decoding(false).is_err() {
        return false;
    }

    let num_chunks = num_values / num_components;
    
    for i in 0..num_chunks {
        let len = tag_decoder.decode_symbol();
        if len == 0 || len > 32 {
            return false;
        }
        let val_idx = i * num_components;
        for j in 0..num_components {
            // Read least significant bits for this value
            let val = match in_buffer.decode_least_significant_bits32(len) {
                Ok(v) => v,
                Err(_) => return false,
            };
            symbols[val_idx + j] = val;
        }
    }

    in_buffer.end_bit_decoding();

    true
}
