//! Draco Core - Fundamental utilities and data types
//!
//! This crate provides the core building blocks for the Draco 3D compression library,
//! including basic data types, error handling, and utility functions.

pub mod data_types;
pub mod error;
pub mod bit_utils;
pub mod math_utils;
pub mod buffer;
pub mod encoder_buffer;
pub mod decoder_buffer;
pub mod vector_extensions;

#[cfg(feature = "c-api")]
pub mod c_api;

// Re-export commonly used types for convenience
pub use data_types::DataType;
pub use error::{Status, StatusResult, ok_status};