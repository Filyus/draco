//! C API layer for Draco core components
//!
//! This module provides C-compatible FFI bindings for the Rust implementation
//! of Draco's core functionality, enabling integration with existing C++ code.

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_uint, c_ulonglong};
use std::ptr;

use crate::bit_utils;
use crate::math_utils;
use crate::buffer;
use crate::encoder_buffer;
use crate::decoder_buffer;

/// Error codes for C API
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum draco_status_t {
    DRACO_STATUS_OK = 0,
    DRACO_STATUS_ERROR = -1,
    DRACO_STATUS_IO_ERROR = -2,
    DRACO_STATUS_INVALID_PARAMETER = -3,
    DRACO_STATUS_UNSUPPORTED_VERSION = -4,
    DRACO_STATUS_UNKNOWN_VERSION = -5,
    DRACO_STATUS_UNSUPPORTED_FEATURE = -6,
}

impl Default for draco_status_t {
    fn default() -> Self {
        draco_status_t::DRACO_STATUS_OK
    }
}

// ===== Bit Utilities C API =====

/// Returns the number of '1' bits within the input 32 bit integer
#[no_mangle]
pub extern "C" fn draco_core_bit_count_ones_32(n: c_uint) -> c_uint {
    bit_utils::count_ones_32(n)
}

/// Returns the number of '1' bits within the input 64 bit integer
#[no_mangle]
pub extern "C" fn draco_core_bit_count_ones_64(n: c_ulonglong) -> c_ulonglong {
    bit_utils::count_ones_64(n)
}

/// Reverses the bits of a 32-bit integer
#[no_mangle]
pub extern "C" fn draco_core_bit_reverse_32(n: c_uint) -> c_uint {
    bit_utils::reverse_bits_32(n)
}

/// Reverses the bits of a 64-bit integer
#[no_mangle]
pub extern "C" fn draco_core_bit_reverse_64(n: c_ulonglong) -> c_ulonglong {
    bit_utils::reverse_bits_64(n)
}

/// Copies `nbits` bits from the src integer into the dst integer using the provided bit offsets (32-bit)
#[no_mangle]
pub extern "C" fn draco_core_bit_copy_32(
    dst: *mut c_uint,
    dst_offset: c_uint,
    src: c_uint,
    src_offset: c_uint,
    nbits: c_uint,
) -> draco_status_t {
    if dst.is_null() {
        set_last_error("Destination pointer is null");
        return draco_status_t::DRACO_STATUS_INVALID_PARAMETER;
    }

    // Validate parameters before calling the function that might panic
    if dst_offset + nbits > 32 || src_offset + nbits > 32 || nbits > 32 {
        return draco_status_t::DRACO_STATUS_INVALID_PARAMETER;
    }

    unsafe {
        bit_utils::copy_bits_32(&mut *dst, dst_offset, src, src_offset, nbits);
    }

    draco_status_t::DRACO_STATUS_OK
}

/// Copies `nbits` bits from the src integer into the dst integer using the provided bit offsets (64-bit)
#[no_mangle]
pub extern "C" fn draco_core_bit_copy_64(
    dst: *mut c_ulonglong,
    dst_offset: c_uint,
    src: c_ulonglong,
    src_offset: c_uint,
    nbits: c_uint,
) -> draco_status_t {
    if dst.is_null() {
        return draco_status_t::DRACO_STATUS_INVALID_PARAMETER;
    }

    // Validate parameters before calling the function that might panic
    if dst_offset + nbits > 64 || src_offset + nbits > 64 || nbits > 64 {
        return draco_status_t::DRACO_STATUS_INVALID_PARAMETER;
    }

    unsafe {
        bit_utils::copy_bits_64(&mut *dst, dst_offset, src, src_offset, nbits);
    }

    draco_status_t::DRACO_STATUS_OK
}

/// Returns the location of the most significant bit in the input integer (32-bit)
#[no_mangle]
pub extern "C" fn draco_core_bit_most_significant_bit(n: c_uint) -> c_uint {
    bit_utils::most_significant_bit(n)
}

/// Returns the location of the most significant bit in the input integer (64-bit)
#[no_mangle]
pub extern "C" fn draco_core_bit_most_significant_bit_64(n: c_ulonglong) -> c_uint {
    bit_utils::most_significant_bit_64(n)
}

/// Converts signed integer to unsigned symbol using zigzag encoding (32-bit)
#[no_mangle]
pub extern "C" fn draco_core_bit_signed_to_symbol_32(val: c_int) -> c_uint {
    bit_utils::convert_signed_int_to_symbol_i32(val)
}

/// Converts unsigned symbol to signed integer using zigzag decoding (32-bit)
#[no_mangle]
pub extern "C" fn draco_core_bit_symbol_to_signed_32(val: c_uint) -> c_int {
    bit_utils::convert_symbol_to_signed_int_i32(val)
}

/// Extracts `count` bits starting from `position` in `value` (32-bit)
#[no_mangle]
pub extern "C" fn draco_core_bit_extract_32(
    value: c_uint,
    position: c_uint,
    count: c_uint,
) -> c_uint {
    bit_utils::extract_bits_32(value, position, count)
}

/// Extracts `count` bits starting from `position` in `value` (64-bit)
#[no_mangle]
pub extern "C" fn draco_core_bit_extract_64(
    value: c_ulonglong,
    position: c_uint,
    count: c_uint,
) -> c_ulonglong {
    bit_utils::extract_bits_64(value, position, count)
}

// ===== Math Utilities C API =====

/// Returns floor(sqrt(x)) where x is an integer number (64-bit)
#[no_mangle]
pub extern "C" fn draco_core_math_int_sqrt(number: c_ulonglong) -> c_ulonglong {
    math_utils::int_sqrt(number)
}

/// Returns floor(sqrt(x)) where x is an integer number (32-bit)
#[no_mangle]
pub extern "C" fn draco_core_math_int_sqrt_32(number: c_uint) -> c_uint {
    math_utils::int_sqrt_32(number)
}

/// Performs addition with wrapping behavior (32-bit)
#[no_mangle]
pub extern "C" fn draco_core_math_add_wrapping_32(a: c_uint, b: c_uint) -> c_uint {
    math_utils::add_wrapping_u32(a, b)
}

/// Performs subtraction with wrapping behavior (32-bit)
#[no_mangle]
pub extern "C" fn draco_core_math_sub_wrapping_32(a: c_uint, b: c_uint) -> c_uint {
    math_utils::sub_wrapping_u32(a, b)
}

/// Performs multiplication with wrapping behavior (32-bit)
#[no_mangle]
pub extern "C" fn draco_core_math_mul_wrapping_32(a: c_uint, b: c_uint) -> c_uint {
    math_utils::mul_wrapping_u32(a, b)
}

/// Performs addition as if the types were unsigned (i32 version)
#[no_mangle]
pub extern "C" fn draco_core_math_add_as_unsigned_32(a: c_int, b: c_int) -> c_int {
    math_utils::add_as_unsigned_i32(a, b)
}

/// Computes the greatest common divisor (GCD) of two integers (i32 version)
#[no_mangle]
pub extern "C" fn draco_core_math_gcd_32(a: c_int, b: c_int) -> c_int {
    math_utils::gcd_i32(a, b)
}

/// Computes the least common multiple (LCM) of two integers (i32 version)
#[no_mangle]
pub extern "C" fn draco_core_math_lcm_32(a: c_int, b: c_int) -> c_int {
    math_utils::lcm_i32(a, b)
}

/// Returns the next power of two greater than or equal to the given value (32-bit)
#[no_mangle]
pub extern "C" fn draco_core_math_next_power_of_two_32(n: c_uint) -> c_uint {
    math_utils::next_power_of_two_u32(n)
}

/// Checks if a number is a power of two (32-bit)
#[no_mangle]
pub extern "C" fn draco_core_math_is_power_of_two_32(n: c_uint) -> bool {
    math_utils::is_power_of_two_u32(n)
}

/// Computes the absolute difference between two values (32-bit)
#[no_mangle]
pub extern "C" fn draco_core_math_abs_diff_32(a: c_uint, b: c_uint) -> c_uint {
    math_utils::abs_diff_u32(a, b)
}

// ===== Buffer Management C API =====

/// Opaque handle for EncoderBuffer
#[repr(C)]
pub struct draco_encoder_buffer_t {
    _private: [u8; 0],
}

/// Opaque handle for DecoderBuffer
#[repr(C)]
pub struct draco_decoder_buffer_t {
    _private: [u8; 0],
}

/// Opaque handle for DataBuffer
#[repr(C)]
pub struct draco_data_buffer_t {
    _private: [u8; 0],
}

/// Creates a new encoder buffer
#[no_mangle]
pub extern "C" fn draco_encoder_buffer_create() -> *mut draco_encoder_buffer_t {
    let buffer = Box::new(encoder_buffer::EncoderBuffer::new());
    Box::into_raw(buffer) as *mut draco_encoder_buffer_t
}

/// Creates a new encoder buffer with initial capacity
#[no_mangle]
pub extern "C" fn draco_encoder_buffer_create_with_capacity(capacity: usize) -> *mut draco_encoder_buffer_t {
    let buffer = Box::new(encoder_buffer::EncoderBuffer::with_capacity(capacity));
    Box::into_raw(buffer) as *mut draco_encoder_buffer_t
}

/// Destroys an encoder buffer
#[no_mangle]
pub extern "C" fn draco_encoder_buffer_destroy(buffer: *mut draco_encoder_buffer_t) {
    if !buffer.is_null() {
        unsafe {
            let _ = Box::from_raw(buffer as *mut encoder_buffer::EncoderBuffer);
        }
    }
}

/// Clears all data from an encoder buffer
#[no_mangle]
pub extern "C" fn draco_encoder_buffer_clear(buffer: *mut draco_encoder_buffer_t) -> draco_status_t {
    if buffer.is_null() {
        return draco_status_t::DRACO_STATUS_INVALID_PARAMETER;
    }

    unsafe {
        let buf = &mut *(buffer as *mut encoder_buffer::EncoderBuffer);
        buf.clear();
    }

    draco_status_t::DRACO_STATUS_OK
}

/// Gets the size of the encoder buffer
#[no_mangle]
pub extern "C" fn draco_encoder_buffer_size(buffer: *const draco_encoder_buffer_t) -> usize {
    if buffer.is_null() {
        return 0;
    }

    unsafe {
        let buf = &*(buffer as *const encoder_buffer::EncoderBuffer);
        buf.size()
    }
}

/// Gets a pointer to the encoder buffer data
#[no_mangle]
pub extern "C" fn draco_encoder_buffer_data(buffer: *const draco_encoder_buffer_t) -> *const u8 {
    if buffer.is_null() {
        return ptr::null();
    }

    unsafe {
        let buf = &*(buffer as *const encoder_buffer::EncoderBuffer);
        buf.data().as_ptr()
    }
}

/// Encodes raw bytes into the encoder buffer
#[no_mangle]
pub extern "C" fn draco_encoder_buffer_encode(
    buffer: *mut draco_encoder_buffer_t,
    data: *const u8,
    size: usize,
) -> draco_status_t {
    if buffer.is_null() || data.is_null() {
        return draco_status_t::DRACO_STATUS_INVALID_PARAMETER;
    }

    unsafe {
        let buf = &mut *(buffer as *mut encoder_buffer::EncoderBuffer);
        let data_slice = std::slice::from_raw_parts(data, size);
        match buf.encode(data_slice) {
            Ok(_) => draco_status_t::DRACO_STATUS_OK,
            Err(e) => {
                set_last_error(e);
                draco_status_t::DRACO_STATUS_ERROR
            }
        }
    }
}

/// Starts bit encoding in the encoder buffer
#[no_mangle]
pub extern "C" fn draco_encoder_buffer_start_bit_encoding(
    buffer: *mut draco_encoder_buffer_t,
    required_bits: usize,
    encode_size: bool,
) -> draco_status_t {
    if buffer.is_null() {
        return draco_status_t::DRACO_STATUS_INVALID_PARAMETER;
    }

    unsafe {
        let buf = &mut *(buffer as *mut encoder_buffer::EncoderBuffer);
        match buf.start_bit_encoding(required_bits, encode_size) {
            Ok(_) => draco_status_t::DRACO_STATUS_OK,
            Err(e) => {
                set_last_error(e);
                draco_status_t::DRACO_STATUS_ERROR
            }
        }
    }
}

/// Encodes least significant bits during bit encoding
#[no_mangle]
pub extern "C" fn draco_encoder_buffer_encode_bits(
    buffer: *mut draco_encoder_buffer_t,
    nbits: c_uint,
    value: c_uint,
) -> draco_status_t {
    if buffer.is_null() || nbits > 32 {
        return draco_status_t::DRACO_STATUS_INVALID_PARAMETER;
    }

    unsafe {
        let buf = &mut *(buffer as *mut encoder_buffer::EncoderBuffer);
        match buf.encode_least_significant_bits_32(nbits as u8, value) {
            Ok(_) => draco_status_t::DRACO_STATUS_OK,
            Err(e) => {
                set_last_error(e);
                draco_status_t::DRACO_STATUS_ERROR
            }
        }
    }
}

/// Ends bit encoding in the encoder buffer
#[no_mangle]
pub extern "C" fn draco_encoder_buffer_end_bit_encoding(buffer: *mut draco_encoder_buffer_t) -> draco_status_t {
    if buffer.is_null() {
        return draco_status_t::DRACO_STATUS_INVALID_PARAMETER;
    }

    unsafe {
        let buf = &mut *(buffer as *mut encoder_buffer::EncoderBuffer);
        buf.end_bit_encoding();
    }

    draco_status_t::DRACO_STATUS_OK
}

/// Checks if bit encoder is active in the encoder buffer
#[no_mangle]
pub extern "C" fn draco_encoder_buffer_bit_encoder_active(buffer: *const draco_encoder_buffer_t) -> bool {
    if buffer.is_null() {
        return false;
    }

    unsafe {
        let buf = &*(buffer as *const encoder_buffer::EncoderBuffer);
        buf.bit_encoder_active()
    }
}

/// Creates a new decoder buffer
#[no_mangle]
pub extern "C" fn draco_decoder_buffer_create() -> *mut draco_decoder_buffer_t {
    let buffer = Box::new(decoder_buffer::DecoderBuffer::new());
    Box::into_raw(buffer) as *mut draco_decoder_buffer_t
}

/// Destroys a decoder buffer
#[no_mangle]
pub extern "C" fn draco_decoder_buffer_destroy(buffer: *mut draco_decoder_buffer_t) {
    if !buffer.is_null() {
        unsafe {
            let _ = Box::from_raw(buffer as *mut decoder_buffer::DecoderBuffer);
        }
    }
}

/// Initializes a decoder buffer with data
#[no_mangle]
pub extern "C" fn draco_decoder_buffer_init(
    buffer: *mut draco_decoder_buffer_t,
    data: *const u8,
    size: usize,
) -> draco_status_t {
    if buffer.is_null() || data.is_null() {
        return draco_status_t::DRACO_STATUS_INVALID_PARAMETER;
    }

    unsafe {
        let buf = &mut *(buffer as *mut decoder_buffer::DecoderBuffer);
        buf.init(data, size);
    }

    draco_status_t::DRACO_STATUS_OK
}

/// Initializes a decoder buffer with data and version
#[no_mangle]
pub extern "C" fn draco_decoder_buffer_init_with_version(
    buffer: *mut draco_decoder_buffer_t,
    data: *const u8,
    size: usize,
    version: c_uint,
) -> draco_status_t {
    if buffer.is_null() || data.is_null() {
        return draco_status_t::DRACO_STATUS_INVALID_PARAMETER;
    }

    unsafe {
        let buf = &mut *(buffer as *mut decoder_buffer::DecoderBuffer);
        buf.init_with_version(data, size, version as u16);
    }

    draco_status_t::DRACO_STATUS_OK
}

/// Gets the current position in the decoder buffer
#[no_mangle]
pub extern "C" fn draco_decoder_buffer_position(buffer: *const draco_decoder_buffer_t) -> usize {
    if buffer.is_null() {
        return 0;
    }

    unsafe {
        let buf = &*(buffer as *const decoder_buffer::DecoderBuffer);
        buf.position()
    }
}

/// Gets the remaining size in the decoder buffer
#[no_mangle]
pub extern "C" fn draco_decoder_buffer_remaining_size(buffer: *const draco_decoder_buffer_t) -> usize {
    if buffer.is_null() {
        return 0;
    }

    unsafe {
        let buf = &*(buffer as *const decoder_buffer::DecoderBuffer);
        buf.remaining_size()
    }
}

/// Starts bit decoding in the decoder buffer
#[no_mangle]
pub extern "C" fn draco_decoder_buffer_start_bit_decoding(
    buffer: *mut draco_decoder_buffer_t,
    decode_size: bool,
    out_size: *mut c_ulonglong,
) -> draco_status_t {
    if buffer.is_null() || out_size.is_null() {
        return draco_status_t::DRACO_STATUS_INVALID_PARAMETER;
    }

    unsafe {
        let buf = &mut *(buffer as *mut decoder_buffer::DecoderBuffer);
        let mut size = 0u64;
        match buf.start_bit_decoding(decode_size, &mut size) {
            Ok(_) => {
                *out_size = size;
                draco_status_t::DRACO_STATUS_OK
            }
            Err(e) => {
                set_last_error(e);
                draco_status_t::DRACO_STATUS_ERROR
            }
        }
    }
}

/// Decodes least significant bits during bit decoding
#[no_mangle]
pub extern "C" fn draco_decoder_buffer_decode_bits(
    buffer: *mut draco_decoder_buffer_t,
    nbits: c_uint,
    out_value: *mut c_uint,
) -> draco_status_t {
    if buffer.is_null() || out_value.is_null() || nbits > 32 {
        return draco_status_t::DRACO_STATUS_INVALID_PARAMETER;
    }

    unsafe {
        let buf = &mut *(buffer as *mut decoder_buffer::DecoderBuffer);
        match buf.decode_least_significant_bits_32(nbits as u8) {
            Ok(value) => {
                *out_value = value;
                draco_status_t::DRACO_STATUS_OK
            }
            Err(e) => {
                set_last_error(e);
                draco_status_t::DRACO_STATUS_ERROR
            }
        }
    }
}

/// Ends bit decoding in the decoder buffer
#[no_mangle]
pub extern "C" fn draco_decoder_buffer_end_bit_decoding(buffer: *mut draco_decoder_buffer_t) -> draco_status_t {
    if buffer.is_null() {
        return draco_status_t::DRACO_STATUS_INVALID_PARAMETER;
    }

    unsafe {
        let buf = &mut *(buffer as *mut decoder_buffer::DecoderBuffer);
        buf.end_bit_decoding();
    }

    draco_status_t::DRACO_STATUS_OK
}

/// Checks if bit decoder is active in the decoder buffer
#[no_mangle]
pub extern "C" fn draco_decoder_buffer_bit_decoder_active(buffer: *const draco_decoder_buffer_t) -> bool {
    if buffer.is_null() {
        return false;
    }

    unsafe {
        let buf = &*(buffer as *const decoder_buffer::DecoderBuffer);
        buf.bit_decoder_active()
    }
}

/// Peeks at data in the decoder buffer without advancing position
#[no_mangle]
pub extern "C" fn draco_decoder_buffer_peek(
    buffer: *const draco_decoder_buffer_t,
    out_data: *mut u8,
    size_to_peek: usize,
) -> draco_status_t {
    if buffer.is_null() || out_data.is_null() {
        return draco_status_t::DRACO_STATUS_INVALID_PARAMETER;
    }

    unsafe {
        let buf = &*(buffer as *const decoder_buffer::DecoderBuffer);
        let data_slice = std::slice::from_raw_parts_mut(out_data, size_to_peek);

        match buf.peek_slice(data_slice) {
            Ok(_) => draco_status_t::DRACO_STATUS_OK,
            Err(_) => draco_status_t::DRACO_STATUS_ERROR,
        }
    }
}

/// Decodes data from the decoder buffer and advances position
#[no_mangle]
pub extern "C" fn draco_decoder_buffer_decode(
    buffer: *mut draco_decoder_buffer_t,
    out_data: *mut u8,
    size_to_decode: usize,
) -> draco_status_t {
    if buffer.is_null() || out_data.is_null() {
        return draco_status_t::DRACO_STATUS_INVALID_PARAMETER;
    }

    unsafe {
        let buf = &mut *(buffer as *mut decoder_buffer::DecoderBuffer);
        let data_slice = std::slice::from_raw_parts_mut(out_data, size_to_decode);

        match buf.decode_slice(data_slice) {
            Ok(_) => draco_status_t::DRACO_STATUS_OK,
            Err(_) => draco_status_t::DRACO_STATUS_ERROR,
        }
    }
}

/// Creates a new data buffer
#[no_mangle]
pub extern "C" fn draco_data_buffer_create_with_capacity(capacity: usize) -> *mut draco_data_buffer_t {
    let buffer = Box::new(buffer::DataBuffer::with_capacity(capacity));
    Box::into_raw(buffer) as *mut draco_data_buffer_t
}

/// Destroys a data buffer
#[no_mangle]
pub extern "C" fn draco_data_buffer_destroy(buffer: *mut draco_data_buffer_t) {
    if !buffer.is_null() {
        unsafe {
            let _ = Box::from_raw(buffer as *mut buffer::DataBuffer);
        }
    }
}

/// Gets the size of the data buffer
#[no_mangle]
pub extern "C" fn draco_data_buffer_size(buffer: *const draco_data_buffer_t) -> usize {
    if buffer.is_null() {
        return 0;
    }

    unsafe {
        let buf = &*(buffer as *const buffer::DataBuffer);
        buf.len()
    }
}

/// Gets a pointer to the data buffer data
#[no_mangle]
pub extern "C" fn draco_data_buffer_data(buffer: *const draco_data_buffer_t) -> *const u8 {
    if buffer.is_null() {
        return ptr::null();
    }

    unsafe {
        let buf = &*(buffer as *const buffer::DataBuffer);
        buf.as_slice().as_ptr()
    }
}

/// Appends data to the data buffer
#[no_mangle]
pub extern "C" fn draco_data_buffer_append(
    buffer: *mut draco_data_buffer_t,
    data: *const u8,
    size: usize,
) -> draco_status_t {
    if buffer.is_null() || data.is_null() {
        return draco_status_t::DRACO_STATUS_INVALID_PARAMETER;
    }

    unsafe {
        let buf = &mut *(buffer as *mut buffer::DataBuffer);
        let data_slice = std::slice::from_raw_parts(data, size);
        buf.extend_from_slice(data_slice);
    }

    draco_status_t::DRACO_STATUS_OK
}

// ===== Error Handling C API =====

thread_local! {
    static LAST_ERROR: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
}

/// Gets the last error message from the Rust implementation
#[no_mangle]
pub extern "C" fn draco_core_get_last_error() -> *const c_char {
    LAST_ERROR.with(|error| {
        if let Some(ref msg) = *error.borrow() {
            match CString::new(msg.as_str()) {
                Ok(s) => s.into_raw() as *const c_char,
                Err(_) => ptr::null(),
            }
        } else {
            ptr::null()
        }
    })
}

/// Clears the last error message
#[no_mangle]
pub extern "C" fn draco_core_clear_error() {
    LAST_ERROR.with(|error| {
        *error.borrow_mut() = None;
    });
}

/// Converts a Rust Status to C status code
fn status_to_c(status: crate::error::Status) -> draco_status_t {
    match status.code() {
        crate::error::ErrorCode::Ok => draco_status_t::DRACO_STATUS_OK,
        crate::error::ErrorCode::DracoError => draco_status_t::DRACO_STATUS_ERROR,
        crate::error::ErrorCode::IoError => draco_status_t::DRACO_STATUS_IO_ERROR,
        crate::error::ErrorCode::InvalidParameter => draco_status_t::DRACO_STATUS_INVALID_PARAMETER,
        crate::error::ErrorCode::UnsupportedVersion => draco_status_t::DRACO_STATUS_UNSUPPORTED_VERSION,
        crate::error::ErrorCode::UnknownVersion => draco_status_t::DRACO_STATUS_UNKNOWN_VERSION,
        crate::error::ErrorCode::UnsupportedFeature => draco_status_t::DRACO_STATUS_UNSUPPORTED_FEATURE,
    }
}

/// Sets the last error message
fn set_last_error(error_msg: &str) {
    LAST_ERROR.with(|last_error| {
        *last_error.borrow_mut() = Some(error_msg.to_string());
    });
}

/// Sets the last error message from a Rust result
fn set_last_error_from_result<T, E: std::fmt::Display>(result: Result<T, E>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            set_last_error(&error.to_string());
            None
        }
    }
}

// ===== Version Information C API =====

/// Returns the version of the Draco core library
#[no_mangle]
pub extern "C" fn draco_core_version() -> *const c_char {
    // Use CStr for static string literals
    let version = std::ffi::CStr::from_bytes_with_nul(b"1.0.0-rust\0").unwrap();
    version.as_ptr()
}

/// Returns a string describing the build configuration
#[no_mangle]
pub extern "C" fn draco_core_build_info() -> *const c_char {
    let info = std::ffi::CStr::from_bytes_with_nul(b"Draco Core Rust Implementation\0").unwrap();
    info.as_ptr()
}

// ===== Initialization and Cleanup C API =====

/// Initialize the Draco core library
/// This must be called before using any other functions from this library
#[no_mangle]
pub extern "C" fn draco_core_init() -> draco_status_t {
    // Initialize any global state here
    draco_status_t::DRACO_STATUS_OK
}

/// Cleanup the Draco core library
/// This should be called when the library is no longer needed
#[no_mangle]
pub extern "C" fn draco_core_cleanup() -> draco_status_t {
    // Cleanup any global state here
    draco_core_clear_error();
    draco_status_t::DRACO_STATUS_OK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_api() {
        assert_eq!(draco_core_bit_count_ones_32(0b1011), 3);
        assert_eq!(draco_core_bit_count_ones_64(0b1011), 3);
        assert_eq!(draco_core_bit_reverse_32(0b0001), 0b10000000000000000000000000000000);

        let mut dst: c_uint = 0;
        let result = draco_core_bit_copy_32(
            &mut dst as *mut c_uint,
            0,
            0b1011,
            0,
            4
        );
        assert_eq!(result, draco_status_t::DRACO_STATUS_OK);
        assert_eq!(dst, 0b1011);
    }

    #[test]
    fn test_math_api() {
        assert_eq!(draco_core_math_int_sqrt(16), 4);
        assert_eq!(draco_core_math_int_sqrt(15), 3);
        assert_eq!(draco_core_math_gcd_32(48, 18), 6);
        assert_eq!(draco_core_math_lcm_32(12, 18), 36);
        assert_eq!(draco_core_math_next_power_of_two_32(5), 8);
        assert!(draco_core_math_is_power_of_two_32(8));
        assert!(!draco_core_math_is_power_of_two_32(5));
    }

    #[test]
    fn test_error_handling() {
        // Test null pointer error
        let result = draco_core_bit_copy_32(
            ptr::null_mut(),
            0,
            0b1011,
            0,
            4
        );
        assert_eq!(result, draco_status_t::DRACO_STATUS_INVALID_PARAMETER);

        let error_ptr = draco_core_get_last_error();
        assert!(!error_ptr.is_null());

        draco_core_clear_error();
        let error_ptr = draco_core_get_last_error();
        assert!(error_ptr.is_null());
    }

    #[test]
    fn test_version_info() {
        let version_ptr = draco_core_version();
        assert!(!version_ptr.is_null());

        let build_info_ptr = draco_core_build_info();
        assert!(!build_info_ptr.is_null());
    }

    #[test]
    fn test_init_cleanup() {
        let result = draco_core_init();
        assert_eq!(result, draco_status_t::DRACO_STATUS_OK);

        let result = draco_core_cleanup();
        assert_eq!(result, draco_status_t::DRACO_STATUS_OK);
    }
}