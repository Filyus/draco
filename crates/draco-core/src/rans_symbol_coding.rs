#[derive(Debug, Clone, Copy, Default)]
pub struct RAnsSymbol {
    pub prob: u32,
    pub cum_prob: u32,
}

pub fn compute_rans_unclamped_precision(symbols_bit_length: u32) -> u32 {
    (3 * symbols_bit_length) / 2
}

pub fn compute_rans_precision_from_unique_symbols_bit_length(symbols_bit_length: u32) -> u32 {
    let prec = compute_rans_unclamped_precision(symbols_bit_length);
    if prec < 12 {
        12
    } else if prec > 20 {
        20
    } else {
        prec
    }
}

pub fn approximate_rans_frequency_table_bits(max_value: u32, num_unique_symbols: u32) -> u64 {
    let diff = if max_value >= num_unique_symbols {
        max_value - num_unique_symbols
    } else {
        0
    };
    let table_zero_frequency_bits = 8 * (num_unique_symbols + diff / 64);
    (8 * num_unique_symbols + table_zero_frequency_bits) as u64
}
