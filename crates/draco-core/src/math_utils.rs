//! Mathematical utilities used throughout Draco
//!
//! This module provides mathematical functions optimized for geometry compression,
//! including integer square root and safe arithmetic operations.

/// Returns floor(sqrt(x)) where x is an integer number
///
/// The main intent of this function is to provide a cross platform and
/// deterministic implementation of square root for integer numbers.
/// This function is not intended to be a replacement for std::sqrt() for
/// general cases. IntSqrt is in fact about 3X slower compared to most
/// implementation of std::sqrt(), but it provides deterministic results
/// across platforms and avoids floating-point precision issues.
///
/// # Arguments
/// * `number` - The number to compute the integer square root of
///
/// # Returns
/// The floor of the square root of `number`
///
/// # Examples
/// ```
/// use draco_core::math_utils::int_sqrt;
/// assert_eq!(int_sqrt(0), 0);
/// assert_eq!(int_sqrt(1), 1);
/// assert_eq!(int_sqrt(4), 2);
/// assert_eq!(int_sqrt(9), 3);
/// assert_eq!(int_sqrt(15), 3); // floor(sqrt(15)) = 3
/// assert_eq!(int_sqrt(16), 4);
/// assert_eq!(int_sqrt(100), 10);
/// ```
pub fn int_sqrt(number: u64) -> u64 {
    if number == 0 {
        return 0;
    }

    // First estimate good initial value of the square root as log2(number)
    let mut act_number = number;
    let mut square_root = 1u64;

    while act_number >= 2 {
        // Double the square root until |square_root * square_root > number|
        square_root *= 2;
        act_number /= 4;
    }

    // Perform Newton's (or Babylonian) method to find the true floor(sqrt())
    loop {
        // New |square_root| estimate is computed as the average between
        // |square_root| and |number / square_root|
        square_root = (square_root + number / square_root) / 2;

        // Note that after the first iteration, the estimate is always going to be
        // larger or equal to the true square root value. Therefore to check
        // convergence, we can simply detect condition when the square of the
        // estimated square root is larger than the input.
        if square_root * square_root <= number {
            break;
        }
    }

    square_root
}

/// 32-bit version of integer square root
pub fn int_sqrt_32(number: u32) -> u32 {
    if number == 0 {
        return 0;
    }

    let mut act_number = number;
    let mut square_root = 1u32;

    while act_number >= 2 {
        square_root *= 2;
        act_number /= 4;
    }

    loop {
        square_root = (square_root + number / square_root) / 2;
        if square_root * square_root <= number {
            break;
        }
    }

    square_root
}

/// Performs addition with overflow detection and wrapping behavior (32-bit)
#[inline]
pub fn add_wrapping_u32(a: u32, b: u32) -> u32 {
    a.wrapping_add(b)
}

/// Performs subtraction with overflow detection and wrapping behavior (32-bit)
#[inline]
pub fn sub_wrapping_u32(a: u32, b: u32) -> u32 {
    a.wrapping_sub(b)
}

/// Performs multiplication with overflow detection and wrapping behavior (32-bit)
#[inline]
pub fn mul_wrapping_u32(a: u32, b: u32) -> u32 {
    a.wrapping_mul(b)
}

/// Performs addition with checked overflow (32-bit)
#[inline]
pub fn add_checked_u32(a: u32, b: u32) -> Option<u32> {
    a.checked_add(b)
}

/// Performs subtraction with checked overflow (32-bit)
#[inline]
pub fn sub_checked_u32(a: u32, b: u32) -> Option<u32> {
    a.checked_sub(b)
}

/// Performs multiplication with checked overflow (32-bit)
#[inline]
pub fn mul_checked_u32(a: u32, b: u32) -> Option<u32> {
    a.checked_mul(b)
}

/// Performs addition as if the types were unsigned (i32 version)
///
/// This is equivalent to the AddAsUnsigned template function in the C++ code.
/// It converts to unsigned, adds, then converts back to the signed type.
///
/// # Examples
/// ```
/// use draco_core::math_utils::add_as_unsigned_i32;
/// let result = add_as_unsigned_i32(100i32, 50i32);
/// assert_eq!(result, 150i32);
/// ```
#[inline]
pub fn add_as_unsigned_i32(a: i32, b: i32) -> i32 {
    ((a as u32).wrapping_add(b as u32)) as i32
}

/// Computes the greatest common divisor (GCD) of two integers using Euclidean algorithm (i32 version)
///
/// # Examples
/// ```
/// use draco_core::math_utils::gcd_i32;
/// assert_eq!(gcd_i32(48, 18), 6);
/// assert_eq!(gcd_i32(17, 13), 1);
/// assert_eq!(gcd_i32(0, 5), 5);
/// assert_eq!(gcd_i32(5, 0), 5);
/// ```
pub fn gcd_i32(mut a: i32, mut b: i32) -> i32 {
    a = a.abs();
    b = b.abs();

    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }

    a
}

/// Computes the least common multiple (LCM) of two integers (i32 version)
///
/// # Examples
/// ```
/// use draco_core::math_utils::lcm_i32;
/// assert_eq!(lcm_i32(12, 18), 36);
/// assert_eq!(lcm_i32(5, 7), 35);
/// assert_eq!(lcm_i32(0, 5), 0);
/// ```
pub fn lcm_i32(a: i32, b: i32) -> i32 {
    if a == 0 || b == 0 {
        return 0;
    }

    (a.abs() / gcd_i32(a, b)) * b.abs()
}

/// Clamps a value between a minimum and maximum
///
/// # Examples
/// ```
/// use draco_core::math_utils::clamp;
/// assert_eq!(clamp(5, 0, 10), 5);
/// assert_eq!(clamp(-5, 0, 10), 0);
/// assert_eq!(clamp(15, 0, 10), 10);
/// ```
#[inline]
pub fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// Linear interpolation between two values
///
/// # Arguments
/// * `a` - Start value
/// * `b` - End value
/// * `t` - Interpolation parameter (0.0 = a, 1.0 = b)
///
/// # Examples
/// ```
/// use draco_core::math_utils::lerp;
/// assert_eq!(lerp(0.0, 10.0, 0.5), 5.0);
/// assert_eq!(lerp(0.0, 10.0, 0.0), 0.0);
/// assert_eq!(lerp(0.0, 10.0, 1.0), 10.0);
/// ```
#[inline]
pub fn lerp<T: num_traits::Float>(a: T, b: T, t: T) -> T {
    a + (b - a) * t
}

/// Returns the next power of two greater than or equal to the given value (32-bit)
///
/// # Examples
/// ```
/// use draco_core::math_utils::next_power_of_two_u32;
/// assert_eq!(next_power_of_two_u32(1), 1);
/// assert_eq!(next_power_of_two_u32(5), 8);
/// assert_eq!(next_power_of_two_u32(16), 16);
/// assert_eq!(next_power_of_two_u32(17), 32);
/// ```
pub fn next_power_of_two_u32(mut n: u32) -> u32 {
    if n <= 1 {
        return 1;
    }

    // Decrement n (only if n is already a power of 2)
    n -= 1;

    // Set all bits to the right of the most significant bit
    n |= n >> 1;
    n |= n >> 2;
    n |= n >> 4;
    n |= n >> 8;
    n |= n >> 16;

    n + 1
}

/// Checks if a number is a power of two (32-bit)
///
/// # Examples
/// ```
/// use draco_core::math_utils::is_power_of_two_u32;
/// assert!(is_power_of_two_u32(1));
/// assert!(is_power_of_two_u32(2));
/// assert!(is_power_of_two_u32(16));
/// assert!(!is_power_of_two_u32(3));
/// assert!(!is_power_of_two_u32(0));
/// ```
#[inline]
pub fn is_power_of_two_u32(n: u32) -> bool {
    n != 0 && (n & (n - 1)) == 0
}

/// Computes the absolute difference between two values (32-bit)
#[inline]
pub fn abs_diff_u32(a: u32, b: u32) -> u32 {
    if a > b {
        a - b
    } else {
        b - a
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int_sqrt() {
        // Test basic cases
        assert_eq!(int_sqrt(0), 0);
        assert_eq!(int_sqrt(1), 1);
        assert_eq!(int_sqrt(4), 2);
        assert_eq!(int_sqrt(9), 3);
        assert_eq!(int_sqrt(16), 4);
        assert_eq!(int_sqrt(25), 5);
        assert_eq!(int_sqrt(36), 6);
        assert_eq!(int_sqrt(49), 7);
        assert_eq!(int_sqrt(64), 8);
        assert_eq!(int_sqrt(81), 9);
        assert_eq!(int_sqrt(100), 10);

        // Test floor behavior
        assert_eq!(int_sqrt(2), 1);
        assert_eq!(int_sqrt(3), 1);
        assert_eq!(int_sqrt(15), 3);
        assert_eq!(int_sqrt(24), 4);
        assert_eq!(int_sqrt(35), 5);

        // Test large numbers
        assert_eq!(int_sqrt(10000), 100);
        assert_eq!(int_sqrt(99999), 316); // floor(sqrt(99999)) = 316
        assert_eq!(int_sqrt(u64::MAX), 4294967295);
    }

    #[test]
    fn test_int_sqrt_32() {
        assert_eq!(int_sqrt_32(0), 0);
        assert_eq!(int_sqrt_32(1), 1);
        assert_eq!(int_sqrt_32(4), 2);
        assert_eq!(int_sqrt_32(9), 3);
        assert_eq!(int_sqrt_32(15), 3);
        assert_eq!(int_sqrt_32(16), 4);
        assert_eq!(int_sqrt_32(100), 10);
        assert_eq!(int_sqrt_32(u32::MAX), 65535);
    }

    #[test]
    fn test_add_as_unsigned() {
        // Test with signed types
        let result = add_as_unsigned_i32(100i32, 50i32);
        assert_eq!(result, 150i32);

        let result = add_as_unsigned_i32(20000i32, 15000i32);
        assert_eq!(result, 35000i32);

        // Test negative numbers
        let result = add_as_unsigned_i32(-5i32, 10i32);
        assert_eq!(result, 5i32);
    }

    #[test]
    fn test_wrapping_operations() {
        assert_eq!(add_wrapping_u32(200, 100), 300); // no wrap in u32
        assert_eq!(add_wrapping_u32(u32::MAX - 1, 2), 0); // wraps around
        assert_eq!(sub_wrapping_u32(50, 100), u32::MAX - 49); // wraps around
        assert_eq!(mul_wrapping_u32(100, 3), 300);
        assert_eq!(mul_wrapping_u32(u32::MAX / 2 + 1, 2), 0); // wraps around to 0
    }

    #[test]
    fn test_checked_operations() {
        assert_eq!(add_checked_u32(100, 50), Some(150));
        assert_eq!(add_checked_u32(u32::MAX - 1, 2), None); // would overflow

        assert_eq!(sub_checked_u32(100, 50), Some(50));
        assert_eq!(sub_checked_u32(50, 100), None); // would overflow

        assert_eq!(mul_checked_u32(100, 2), Some(200));
        assert_eq!(mul_checked_u32(u32::MAX / 2 + 1, 2), None); // would overflow
    }

    #[test]
    fn test_gcd() {
        assert_eq!(gcd_i32(48, 18), 6);
        assert_eq!(gcd_i32(18, 48), 6); // commutative
        assert_eq!(gcd_i32(17, 13), 1); // co-prime
        assert_eq!(gcd_i32(0, 5), 5);
        assert_eq!(gcd_i32(5, 0), 5);
        assert_eq!(gcd_i32(0, 0), 0);
        assert_eq!(gcd_i32(-12, 18), 6); // handles negative
        assert_eq!(gcd_i32(12, -18), 6); // handles negative
        assert_eq!(gcd_i32(-12, -18), 6); // handles negative
    }

    #[test]
    fn test_lcm() {
        assert_eq!(lcm_i32(12, 18), 36);
        assert_eq!(lcm_i32(5, 7), 35);
        assert_eq!(lcm_i32(0, 5), 0);
        assert_eq!(lcm_i32(5, 0), 0);
        assert_eq!(lcm_i32(0, 0), 0);
    }

    #[test]
    fn test_clamp() {
        assert_eq!(clamp(5, 0, 10), 5);
        assert_eq!(clamp(-5, 0, 10), 0);
        assert_eq!(clamp(15, 0, 10), 10);
        assert_eq!(clamp(0, 0, 10), 0);
        assert_eq!(clamp(10, 0, 10), 10);
    }

    #[test]
    fn test_lerp() {
        assert_eq!(lerp(0.0, 10.0, 0.5), 5.0);
        assert_eq!(lerp(0.0, 10.0, 0.0), 0.0);
        assert_eq!(lerp(0.0, 10.0, 1.0), 10.0);
        assert_eq!(lerp(5.0, 15.0, 0.5), 10.0);
    }

    #[test]
    fn test_next_power_of_two() {
        assert_eq!(next_power_of_two_u32(0), 1);
        assert_eq!(next_power_of_two_u32(1), 1);
        assert_eq!(next_power_of_two_u32(2), 2);
        assert_eq!(next_power_of_two_u32(3), 4);
        assert_eq!(next_power_of_two_u32(5), 8);
        assert_eq!(next_power_of_two_u32(16), 16);
        assert_eq!(next_power_of_two_u32(17), 32);
    }

    #[test]
    fn test_is_power_of_two() {
        assert!(is_power_of_two_u32(1));
        assert!(is_power_of_two_u32(2));
        assert!(is_power_of_two_u32(4));
        assert!(is_power_of_two_u32(8));
        assert!(is_power_of_two_u32(16));
        assert!(!is_power_of_two_u32(3));
        assert!(!is_power_of_two_u32(5));
        assert!(!is_power_of_two_u32(0));
    }

    #[test]
    fn test_abs_diff() {
        assert_eq!(abs_diff_u32(10, 5), 5);
        assert_eq!(abs_diff_u32(5, 10), 5);
        assert_eq!(abs_diff_u32(10, 10), 0);
    }

    #[test]
    fn test_large_numbers() {
        // Test with large values to ensure no overflow issues
        let large_num = u64::MAX / 4;
        let sqrt_result = int_sqrt(large_num);
        assert!(sqrt_result * sqrt_result <= large_num);
        assert!((sqrt_result + 1) * (sqrt_result + 1) > large_num);
    }
}