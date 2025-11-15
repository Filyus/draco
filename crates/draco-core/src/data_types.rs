//! Core data types used throughout Draco
//!
//! This module provides the fundamental data type definitions used by the compression
//! algorithms, mirroring the C++ Draco DataType enum while providing Rust-specific
//! optimizations and safety guarantees.

use std::fmt;

/// Draco data types for geometry attributes and compression
///
/// This enum represents the different data types that can be stored in Draco
/// compressed files, corresponding to the original C++ DataType enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DataType {
    /// Not a legal value for DataType. Used to indicate a field has not been set.
    Invalid = 0,
    /// 8-bit signed integer
    Int8 = 1,
    /// 8-bit unsigned integer
    UInt8 = 2,
    /// 16-bit signed integer
    Int16 = 3,
    /// 16-bit unsigned integer
    UInt16 = 4,
    /// 32-bit signed integer
    Int32 = 5,
    /// 32-bit unsigned integer
    UInt32 = 6,
    /// 64-bit signed integer
    Int64 = 7,
    /// 64-bit unsigned integer
    UInt64 = 8,
    /// 32-bit floating point number
    Float32 = 9,
    /// 64-bit floating point number
    Float64 = 10,
    /// Boolean value
    Bool = 11,
}

impl DataType {
    /// Returns the size of this data type in bytes
    ///
    /// # Examples
    /// ```
    /// use draco_core::data_types::DataType;
    /// assert_eq!(DataType::Int32.size(), 4);
    /// assert_eq!(DataType::Float64.size(), 8);
    /// assert_eq!(DataType::Bool.size(), 1);
    /// ```
    pub const fn size(self) -> usize {
        match self {
            DataType::Invalid => 0,
            DataType::Int8 | DataType::UInt8 | DataType::Bool => 1,
            DataType::Int16 | DataType::UInt16 => 2,
            DataType::Int32 | DataType::UInt32 | DataType::Float32 => 4,
            DataType::Int64 | DataType::UInt64 | DataType::Float64 => 8,
        }
    }

    /// Returns the name of this data type as a string
    pub const fn name(self) -> &'static str {
        match self {
            DataType::Invalid => "invalid",
            DataType::Int8 => "int8",
            DataType::UInt8 => "uint8",
            DataType::Int16 => "int16",
            DataType::UInt16 => "uint16",
            DataType::Int32 => "int32",
            DataType::UInt32 => "uint32",
            DataType::Int64 => "int64",
            DataType::UInt64 => "uint64",
            DataType::Float32 => "float32",
            DataType::Float64 => "float64",
            DataType::Bool => "bool",
        }
    }

    /// Returns true if this is an integral data type (including boolean)
    ///
    /// Equivalent to std::is_integral for draco::DataType. Returns true for all
    /// signed and unsigned integer types (including DT_BOOL).
    pub const fn is_integral(self) -> bool {
        matches!(
            self,
            DataType::Int8
                | DataType::UInt8
                | DataType::Int16
                | DataType::UInt16
                | DataType::Int32
                | DataType::UInt32
                | DataType::Int64
                | DataType::UInt64
                | DataType::Bool
        )
    }

    /// Returns true if this is a floating point data type
    pub const fn is_floating_point(self) -> bool {
        matches!(self, DataType::Float32 | DataType::Float64)
    }

    /// Returns true if this is a signed integer type
    pub const fn is_signed_integer(self) -> bool {
        matches!(
            self,
            DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64
        )
    }

    /// Returns true if this is an unsigned integer type
    pub const fn is_unsigned_integer(self) -> bool {
        matches!(
            self,
            DataType::UInt8 | DataType::UInt16 | DataType::UInt32 | DataType::UInt64
        )
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Default for DataType {
    fn default() -> Self {
        DataType::Invalid
    }
}

/// Creates a DataType from a u8 value
///
/// Returns DataType::Invalid for any value that doesn't correspond to a valid type
impl From<u8> for DataType {
    fn from(value: u8) -> Self {
        match value {
            0 => DataType::Invalid,
            1 => DataType::Int8,
            2 => DataType::UInt8,
            3 => DataType::Int16,
            4 => DataType::UInt16,
            5 => DataType::Int32,
            6 => DataType::UInt32,
            7 => DataType::Int64,
            8 => DataType::UInt64,
            9 => DataType::Float32,
            10 => DataType::Float64,
            11 => DataType::Bool,
            _ => DataType::Invalid,
        }
    }
}

/// Creates a DataType from a string
///
/// Case-insensitive conversion from string representation
impl From<&str> for DataType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "int8" => DataType::Int8,
            "uint8" => DataType::UInt8,
            "int16" => DataType::Int16,
            "uint16" => DataType::UInt16,
            "int32" => DataType::Int32,
            "uint32" => DataType::UInt32,
            "int64" => DataType::Int64,
            "uint64" => DataType::UInt64,
            "float32" => DataType::Float32,
            "float64" => DataType::Float64,
            "bool" => DataType::Bool,
            _ => DataType::Invalid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_type_size() {
        assert_eq!(DataType::Int8.size(), 1);
        assert_eq!(DataType::UInt8.size(), 1);
        assert_eq!(DataType::Int16.size(), 2);
        assert_eq!(DataType::UInt16.size(), 2);
        assert_eq!(DataType::Int32.size(), 4);
        assert_eq!(DataType::UInt32.size(), 4);
        assert_eq!(DataType::Int64.size(), 8);
        assert_eq!(DataType::UInt64.size(), 8);
        assert_eq!(DataType::Float32.size(), 4);
        assert_eq!(DataType::Float64.size(), 8);
        assert_eq!(DataType::Bool.size(), 1);
        assert_eq!(DataType::Invalid.size(), 0);
    }

    #[test]
    fn test_is_integral() {
        assert!(DataType::Int8.is_integral());
        assert!(DataType::UInt8.is_integral());
        assert!(DataType::Int16.is_integral());
        assert!(DataType::UInt16.is_integral());
        assert!(DataType::Int32.is_integral());
        assert!(DataType::UInt32.is_integral());
        assert!(DataType::Int64.is_integral());
        assert!(DataType::UInt64.is_integral());
        assert!(DataType::Bool.is_integral());

        assert!(!DataType::Float32.is_integral());
        assert!(!DataType::Float64.is_integral());
    }

    #[test]
    fn test_is_floating_point() {
        assert!(DataType::Float32.is_floating_point());
        assert!(DataType::Float64.is_floating_point());

        assert!(!DataType::Int8.is_floating_point());
        assert!(!DataType::UInt8.is_floating_point());
        assert!(!DataType::Bool.is_floating_point());
    }

    #[test]
    fn test_from_u8() {
        assert_eq!(DataType::from(1u8), DataType::Int8);
        assert_eq!(DataType::from(2u8), DataType::UInt8);
        assert_eq!(DataType::from(11u8), DataType::Bool);
        assert_eq!(DataType::from(0u8), DataType::Invalid);
        assert_eq!(DataType::from(255u8), DataType::Invalid);
    }

    #[test]
    fn test_from_str() {
        assert_eq!(DataType::from("int8"), DataType::Int8);
        assert_eq!(DataType::from("UINT8"), DataType::UInt8);
        assert_eq!(DataType::from("Float32"), DataType::Float32);
        assert_eq!(DataType::from("bool"), DataType::Bool);
        assert_eq!(DataType::from("unknown"), DataType::Invalid);
    }
}