//! Bit manipulation utilities
//!
//! This module provides bit manipulation functions used throughout the Draco compression
//! algorithms, optimized for performance while maintaining safety.

/// Returns the number of '1' bits within the input 32 bit integer
///
/// This uses a highly optimized algorithm that counts bits in parallel.
/// The functionality is equivalent to the builtin popcount instruction on many CPUs.
///
/// # Examples
/// ```
/// use draco_core::bit_utils::count_ones_32;
/// assert_eq!(count_ones_32(0b1011), 3);
/// assert_eq!(count_ones_32(0), 0);
/// assert_eq!(count_ones_32(0xFFFFFFFF), 32);
/// ```
#[inline]
pub fn count_ones_32(mut n: u32) -> u32 {
    n = n.wrapping_sub((n >> 1) & 0x55555555);
    n = (n & 0x33333333) + ((n >> 2) & 0x33333333);
    (((n + (n >> 4)) & 0x0F0F0F0F).wrapping_mul(0x01010101)) >> 24
}

/// Returns the number of '1' bits within the input 64 bit integer
#[inline]
pub fn count_ones_64(mut n: u64) -> u64 {
    n = n.wrapping_sub((n >> 1) & 0x5555555555555555);
    n = (n & 0x3333333333333333) + ((n >> 2) & 0x3333333333333333);
    n = (n + (n >> 4)) & 0x0F0F0F0F0F0F0F0F;
    ((n.wrapping_mul(0x0101010101010101)) >> 56) & 0xFF
}

/// Reverses the bits of a 32-bit integer
///
/// # Examples
/// ```
/// use draco_core::bit_utils::reverse_bits_32;
/// assert_eq!(reverse_bits_32(0b0001), 0b10000000000000000000000000000000);
/// assert_eq!(reverse_bits_32(0b1011), 0b11010000000000000000000000000000);
/// ```
#[inline]
pub fn reverse_bits_32(mut n: u32) -> u32 {
    n = ((n >> 1) & 0x55555555) | ((n & 0x55555555) << 1);
    n = ((n >> 2) & 0x33333333) | ((n & 0x33333333) << 2);
    n = ((n >> 4) & 0x0F0F0F0F) | ((n & 0x0F0F0F0F) << 4);
    n = ((n >> 8) & 0x00FF00FF) | ((n & 0x00FF00FF) << 8);
    (n >> 16) | (n << 16)
}

/// Reverses the bits of a 64-bit integer
#[inline]
pub fn reverse_bits_64(mut n: u64) -> u64 {
    n = ((n >> 1) & 0x5555555555555555) | ((n & 0x5555555555555555) << 1);
    n = ((n >> 2) & 0x3333333333333333) | ((n & 0x3333333333333333) << 2);
    n = ((n >> 4) & 0x0F0F0F0F0F0F0F0F) | ((n & 0x0F0F0F0F0F0F0F0F) << 4);
    n = ((n >> 8) & 0x00FF00FF00FF00FF) | ((n & 0x00FF00FF00FF00FF) << 8);
    n = ((n >> 16) & 0x0000FFFF0000FFFF) | ((n & 0x0000FFFF0000FFFF) << 16);
    (n >> 32) | (n << 32)
}

/// Copies `nbits` bits from the src integer into the dst integer using the provided bit offsets
///
/// # Arguments
/// * `dst` - The destination value to copy bits into
/// * `dst_offset` - The bit offset in the destination
/// * `src` - The source value to copy bits from
/// * `src_offset` - The bit offset in the source
/// * `nbits` - The number of bits to copy
///
/// # Panics
/// Panics if dst_offset + nbits > 32 or src_offset + nbits > 32
///
/// # Examples
/// ```
/// use draco_core::bit_utils::copy_bits_32;
/// let mut dst = 0b0000;
/// copy_bits_32(&mut dst, 0, 0b1011, 0, 4);
/// assert_eq!(dst, 0b1011); // All 4 bits copied to dst
/// ```
#[inline]
pub fn copy_bits_32(dst: &mut u32, dst_offset: u32, src: u32, src_offset: u32, nbits: u32) {
    assert!(dst_offset + nbits <= 32, "Destination bit range exceeds 32 bits");
    assert!(src_offset + nbits <= 32, "Source bit range exceeds 32 bits");
    assert!(nbits <= 32, "Cannot copy more than 32 bits");

    if nbits == 0 {
        return;
    }

    let mask = if nbits == 32 {
        0xFFFFFFFF
    } else {
        ((1u32 << nbits) - 1) << dst_offset
    };

    *dst = (*dst & !mask) | (((src >> src_offset) << dst_offset) & mask);
}

/// Copies `nbits` bits from the src integer into the dst integer using the provided bit offsets (64-bit version)
#[inline]
pub fn copy_bits_64(dst: &mut u64, dst_offset: u32, src: u64, src_offset: u32, nbits: u32) {
    assert!(dst_offset + nbits <= 64, "Destination bit range exceeds 64 bits");
    assert!(src_offset + nbits <= 64, "Source bit range exceeds 64 bits");
    assert!(nbits <= 64, "Cannot copy more than 64 bits");

    if nbits == 0 {
        return;
    }

    let mask = if nbits == 64 {
        0xFFFFFFFFFFFFFFFF
    } else {
        ((1u64 << nbits) - 1) << dst_offset
    };

    *dst = (*dst & !mask) | (((src >> src_offset) << dst_offset) & mask);
}

/// Returns the location of the most significant bit in the input integer
///
/// The functionality is not defined for n == 0 and will return u32::MAX in that case.
///
/// # Examples
/// ```
/// use draco_core::bit_utils::most_significant_bit;
/// assert_eq!(most_significant_bit(1), 0);
/// assert_eq!(most_significant_bit(8), 3);
/// assert_eq!(most_significant_bit(0xFF), 7);
/// assert_eq!(most_significant_bit(0), u32::MAX);
/// ```
#[inline]
pub fn most_significant_bit(n: u32) -> u32 {
    if n == 0 {
        return u32::MAX;
    }

    // Use leading_zeros function which is often optimized to a single instruction
    31 - n.leading_zeros()
}

/// Returns the location of the most significant bit in the input 64-bit integer
#[inline]
pub fn most_significant_bit_64(n: u64) -> u32 {
    if n == 0 {
        return u32::MAX;
    }

    63 - n.leading_zeros()
}

/// Helper function that converts signed integer values into unsigned integer symbols
/// that can be encoded using an entropy encoder
///
/// This uses zigzag encoding: positive values are doubled, negative values are transformed
/// to odd numbers to create a more compressible distribution.
///
/// # Examples
/// ```
/// use draco_core::bit_utils::convert_signed_int_to_symbol_i32;
/// assert_eq!(convert_signed_int_to_symbol_i32(0i32), 0u32);
/// assert_eq!(convert_signed_int_to_symbol_i32(1i32), 2u32);
/// assert_eq!(convert_signed_int_to_symbol_i32(-1i32), 1u32);
/// assert_eq!(convert_signed_int_to_symbol_i32(-2i32), 3u32);
/// ```
#[inline]
pub fn convert_signed_int_to_symbol_i32(val: i32) -> u32 {
    // Early exit if val is positive
    if val >= 0 {
        (val as u32) << 1
    } else {
        // Map -1 to 0, -2 to 1, etc., then shift and set LSB
        let neg_val = -(val + 1);
        ((neg_val as u32) << 1) | 1
    }
}

/// Converts a single unsigned integer symbol encoded with an entropy encoder
/// back to a signed value
///
/// This reverses the zigzag encoding performed by convert_signed_int_to_symbol_i32.
///
/// # Examples
/// ```
/// use draco_core::bit_utils::convert_symbol_to_signed_int_i32;
/// assert_eq!(convert_symbol_to_signed_int_i32(0u32), 0i32);
/// assert_eq!(convert_symbol_to_signed_int_i32(2u32), 1i32);
/// assert_eq!(convert_symbol_to_signed_int_i32(1u32), -1i32);
/// assert_eq!(convert_symbol_to_signed_int_i32(3u32), -2i32);
/// ```
#[inline]
pub fn convert_symbol_to_signed_int_i32(mut val: u32) -> i32 {
    let is_positive = (val & 1) == 0;
    val >>= 1;

    if is_positive {
        val as i32
    } else {
        // -(val) - 1
        -(val as i32) - 1
    }
}

/// Converts an array of signed integers to unsigned symbols
pub fn convert_signed_ints_to_symbols(input: &[i32], output: &mut [u32]) {
    assert_eq!(input.len(), output.len(), "Input and output arrays must have the same length");

    for (i, &val) in input.iter().enumerate() {
        output[i] = convert_signed_int_to_symbol_i32(val);
    }
}

/// Converts an array of unsigned symbols to signed integers
pub fn convert_symbols_to_signed_ints(input: &[u32], output: &mut [i32]) {
    assert_eq!(input.len(), output.len(), "Input and output arrays must have the same length");

    for (i, &val) in input.iter().enumerate() {
        output[i] = convert_symbol_to_signed_int_i32(val);
    }
}

/// Returns true if the bit at position `bit` is set in `value` (32-bit)
#[inline]
pub fn is_bit_set_32(value: u32, bit: u32) -> bool {
    (value & (1u32 << bit)) != 0
}

/// Sets the bit at position `bit` in `value` (32-bit)
#[inline]
pub fn set_bit_32(value: u32, bit: u32) -> u32 {
    value | (1u32 << bit)
}

/// Clears the bit at position `bit` in `value` (32-bit)
#[inline]
pub fn clear_bit_32(value: u32, bit: u32) -> u32 {
    value & !(1u32 << bit)
}

/// Toggles the bit at position `bit` in `value` (32-bit)
#[inline]
pub fn toggle_bit_32(value: u32, bit: u32) -> u32 {
    value ^ (1u32 << bit)
}

/// Extracts `count` bits starting from `position` in `value`
#[inline]
pub fn extract_bits_32(value: u32, position: u32, count: u32) -> u32 {
    if count == 0 {
        return 0;
    }

    let mask = if count >= 32 {
        0xFFFFFFFF
    } else {
        (1u32 << count) - 1
    };

    (value >> position) & mask
}

/// Extracts `count` bits starting from `position` in `value` (64-bit version)
#[inline]
pub fn extract_bits_64(value: u64, position: u32, count: u32) -> u64 {
    if count == 0 {
        return 0;
    }

    let mask = if count >= 64 {
        0xFFFFFFFFFFFFFFFF
    } else {
        (1u64 << count) - 1
    };

    (value >> position) & mask
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_ones_32() {
        assert_eq!(count_ones_32(0b0), 0);
        assert_eq!(count_ones_32(0b1), 1);
        assert_eq!(count_ones_32(0b1011), 3);
        assert_eq!(count_ones_32(0xFFFFFFFF), 32);
        assert_eq!(count_ones_32(0xAAAAAAAA), 16);
        assert_eq!(count_ones_32(0x55555555), 16);
    }

    #[test]
    fn test_count_ones_64() {
        assert_eq!(count_ones_64(0b0), 0);
        assert_eq!(count_ones_64(0b1), 1);
        assert_eq!(count_ones_64(0b1011), 3);
        assert_eq!(count_ones_64(0xFFFFFFFFFFFFFFFF), 64);
        assert_eq!(count_ones_64(0xAAAAAAAAAAAAAAAA), 32);
        assert_eq!(count_ones_64(0x5555555555555555), 32);
    }

    #[test]
    fn test_reverse_bits_32() {
        assert_eq!(reverse_bits_32(0b0001), 0b10000000000000000000000000000000);
        assert_eq!(reverse_bits_32(0b1011), 0b11010000000000000000000000000000);
        assert_eq!(reverse_bits_32(0), 0);
        assert_eq!(reverse_bits_32(0xFFFFFFFF), 0xFFFFFFFF);
        assert_eq!(reverse_bits_32(0x80000001), 0x80000001); // Symmetric case
    }

    #[test]
    fn test_reverse_bits_64() {
        assert_eq!(reverse_bits_64(0b0001), 0b1000000000000000000000000000000000000000000000000000000000000000);
        assert_eq!(reverse_bits_64(0), 0);
        assert_eq!(reverse_bits_64(0xFFFFFFFFFFFFFFFF), 0xFFFFFFFFFFFFFFFF);
    }

    #[test]
    fn test_copy_bits_32() {
        // Simple test: copy all bits
        let mut dst = 0;
        copy_bits_32(&mut dst, 0, 0b1011, 0, 4); // Copy all 4 bits
        assert_eq!(dst, 0b1011);

        // Test edge case: full copy
        let mut dst = 0;
        copy_bits_32(&mut dst, 0, 0xFFFFFFFF, 0, 32);
        assert_eq!(dst, 0xFFFFFFFF);
    }

    #[test]
    fn test_most_significant_bit() {
        assert_eq!(most_significant_bit(1), 0);
        assert_eq!(most_significant_bit(2), 1);
        assert_eq!(most_significant_bit(8), 3);
        assert_eq!(most_significant_bit(0xFF), 7);
        assert_eq!(most_significant_bit(0x8000), 15);
        assert_eq!(most_significant_bit(0), u32::MAX);
    }

    #[test]
    fn test_zigzag_conversion() {
        // Test 32-bit
        assert_eq!(convert_signed_int_to_symbol_i32(0i32), 0u32);
        assert_eq!(convert_signed_int_to_symbol_i32(1i32), 2u32);
        assert_eq!(convert_signed_int_to_symbol_i32(-1i32), 1u32);
        assert_eq!(convert_signed_int_to_symbol_i32(2i32), 4u32);
        assert_eq!(convert_signed_int_to_symbol_i32(-2i32), 3u32);

        assert_eq!(convert_symbol_to_signed_int_i32(0u32), 0i32);
        assert_eq!(convert_symbol_to_signed_int_i32(2u32), 1i32);
        assert_eq!(convert_symbol_to_signed_int_i32(1u32), -1i32);
        assert_eq!(convert_symbol_to_signed_int_i32(4u32), 2i32);
        assert_eq!(convert_symbol_to_signed_int_i32(3u32), -2i32);

        // Test round-trip
        for i in -1000..=1000 {
            let symbol = convert_signed_int_to_symbol_i32(i);
            let restored = convert_symbol_to_signed_int_i32(symbol);
            assert_eq!(i, restored);
        }
    }

    #[test]
    fn test_array_conversions() {
        let input = vec![-3, -2, -1, 0, 1, 2, 3];
        let expected_symbols = vec![5, 3, 1, 0, 2, 4, 6];

        let mut output = vec![0u32; input.len()];
        convert_signed_ints_to_symbols(&input, &mut output);
        assert_eq!(output, expected_symbols);

        let mut restored = vec![0i32; input.len()];
        convert_symbols_to_signed_ints(&output, &mut restored);
        assert_eq!(restored, input);
    }

    #[test]
    fn test_bit_operations() {
        assert_eq!(extract_bits_32(0b1011_1100u32, 2, 3), 0b111);
        assert_eq!(extract_bits_32(0b1011_1100u32, 0, 4), 0b1100);

        assert!(is_bit_set_32(0b1010, 1));
        assert!(!is_bit_set_32(0b1010, 0));

        assert_eq!(set_bit_32(0b0100, 1), 0b0110);
        assert_eq!(clear_bit_32(0b0110, 1), 0b0100);
        assert_eq!(toggle_bit_32(0b0100, 1), 0b0110);
        assert_eq!(toggle_bit_32(0b0110, 1), 0b0100);
    }

    #[test]
    #[should_panic(expected = "Destination bit range exceeds 32 bits")]
    fn test_copy_bits_32_overflow() {
        let mut dst = 0;
        copy_bits_32(&mut dst, 30, 0b11, 0, 5); // 30 + 5 > 32
    }

    #[test]
    fn test_copy_bits_32_edge_cases() {
        let mut dst = 0;
        copy_bits_32(&mut dst, 0, 0xFFFFFFFF, 0, 0); // 0 bits - should do nothing
        assert_eq!(dst, 0);

        copy_bits_32(&mut dst, 0, 0xFFFFFFFF, 0, 32); // 32 bits
        assert_eq!(dst, 0xFFFFFFFF);
    }
}