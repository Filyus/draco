//! Buffer management utilities
//!
//! This module provides safe buffer management for Draco operations,
//! including encoder and decoder buffers.

use crate::error::{DracoError, StatusResult};

/// Maximum buffer size limit (1GB)
pub const BUFFER_SIZE_LIMIT: usize = 1 << 30;

// Thread-local buffer pointer for C API integration
// Note: Documentation on thread_local! macro invocations is not supported by rustdoc
thread_local! {
    static BUFFER_PTR: std::cell::RefCell<*mut u8> = std::cell::RefCell::new(std::ptr::null_mut());
}

/// Get the current buffer pointer for bit encoding
pub fn get_buffer_ptr() -> *mut u8 {
    BUFFER_PTR.with(|ptr| *ptr.borrow())
}

/// Set the buffer pointer for bit encoding
/// # Safety
/// This function is unsafe and should only be used by the C API layer
pub unsafe fn set_buffer_ptr(ptr: *mut u8) {
    BUFFER_PTR.with(|cell| *cell.borrow_mut() = ptr);
}

/// A generic buffer for Draco operations
///
/// This provides a safe interface for managing raw data buffers used throughout
/// the compression and decompression process.
#[derive(Debug, Clone)]
pub struct DataBuffer {
    data: Vec<u8>,
    position: usize,
}

impl DataBuffer {
    /// Creates a new empty buffer with the specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            position: 0,
        }
    }

    /// Creates a new buffer from existing data
    pub fn from_vec(data: Vec<u8>) -> Self {
        Self {
            data,
            position: 0,
        }
    }

    /// Returns the current size of the buffer
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the current read position
    pub fn position(&self) -> usize {
        self.position
    }

    /// Sets the read position
    pub fn set_position(&mut self, position: usize) -> StatusResult<()> {
        if position > self.data.len() {
            return Err(DracoError::invalid_parameter(format!(
                "Position {} exceeds buffer length {}",
                position,
                self.data.len()
            )));
        }
        self.position = position;
        Ok(())
    }

    /// Resets the position to the beginning of the buffer
    pub fn rewind(&mut self) {
        self.position = 0;
    }

    /// Clears the buffer and resets position
    pub fn clear(&mut self) {
        self.data.clear();
        self.position = 0;
    }

    /// Appends a single byte to the buffer
    pub fn push(&mut self, byte: u8) {
        self.data.push(byte);
    }

    /// Appends a slice of bytes to the buffer
    pub fn extend_from_slice(&mut self, slice: &[u8]) {
        self.data.extend_from_slice(slice);
    }

    /// Reserves capacity for at least `additional` more bytes
    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional);
    }

    /// Gets a slice of the buffer data
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    /// Gets a mutable slice of the buffer data
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Reads a single byte from the current position
    pub fn read_byte(&mut self) -> StatusResult<u8> {
        if self.position >= self.data.len() {
            return Err(DracoError::io_error("Attempted to read past end of buffer"));
        }
        let byte = self.data[self.position];
        self.position += 1;
        Ok(byte)
    }

    /// Reads a slice of bytes from the current position
    pub fn read_slice(&mut self, len: usize) -> StatusResult<&[u8]> {
        if self.position + len > self.data.len() {
            return Err(DracoError::io_error("Attempted to read past end of buffer"));
        }
        let slice = &self.data[self.position..self.position + len];
        self.position += len;
        Ok(slice)
    }

    /// Reads bytes into the provided slice
    pub fn read_into(&mut self, buf: &mut [u8]) -> StatusResult<()> {
        if self.position + buf.len() > self.data.len() {
            return Err(DracoError::io_error("Attempted to read past end of buffer"));
        }
        buf.copy_from_slice(&self.data[self.position..self.position + buf.len()]);
        self.position += buf.len();
        Ok(())
    }

    /// Consumes the buffer and returns the underlying Vec
    pub fn into_vec(self) -> Vec<u8> {
        self.data
    }
}

impl Default for DataBuffer {
    fn default() -> Self {
        Self::with_capacity(0)
    }
}

impl From<Vec<u8>> for DataBuffer {
    fn from(data: Vec<u8>) -> Self {
        Self::from_vec(data)
    }
}

impl From<&[u8]> for DataBuffer {
    fn from(data: &[u8]) -> Self {
        Self::from_vec(data.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_buffer_creation() {
        let buffer = DataBuffer::with_capacity(10);
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
        assert_eq!(buffer.position(), 0);
    }

    #[test]
    fn test_data_buffer_from_vec() {
        let data = vec![1, 2, 3, 4, 5];
        let buffer = DataBuffer::from_vec(data.clone());
        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer.as_slice(), &data[..]);
    }

    #[test]
    fn test_data_buffer_operations() {
        let mut buffer = DataBuffer::with_capacity(10);

        // Test push and extend
        buffer.push(1);
        buffer.push(2);
        buffer.extend_from_slice(&[3, 4, 5]);

        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer.as_slice(), &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_data_buffer_position() {
        let data = vec![1, 2, 3, 4, 5];
        let mut buffer = DataBuffer::from_vec(data);

        assert_eq!(buffer.position(), 0);

        // Test reading bytes
        assert_eq!(buffer.read_byte().unwrap(), 1);
        assert_eq!(buffer.position(), 1);

        assert_eq!(buffer.read_byte().unwrap(), 2);
        assert_eq!(buffer.position(), 2);
    }

    #[test]
    fn test_data_buffer_read_slice() {
        let data = vec![1, 2, 3, 4, 5];
        let mut buffer = DataBuffer::from_vec(data);

        let slice = buffer.read_slice(3).unwrap();
        assert_eq!(slice, &[1, 2, 3]);
        assert_eq!(buffer.position(), 3);

        let remaining = buffer.read_slice(2).unwrap();
        assert_eq!(remaining, &[4, 5]);
        assert_eq!(buffer.position(), 5);
    }

    #[test]
    fn test_data_buffer_read_into() {
        let data = vec![1, 2, 3, 4, 5];
        let mut buffer = DataBuffer::from_vec(data);

        let mut buf = [0u8; 3];
        buffer.read_into(&mut buf).unwrap();
        assert_eq!(buf, [1, 2, 3]);
        assert_eq!(buffer.position(), 3);
    }

    #[test]
    fn test_data_buffer_rewind() {
        let data = vec![1, 2, 3, 4, 5];
        let mut buffer = DataBuffer::from_vec(data);

        buffer.read_byte().unwrap();
        assert_eq!(buffer.position(), 1);

        buffer.rewind();
        assert_eq!(buffer.position(), 0);
    }

    #[test]
    fn test_data_buffer_set_position() {
        let data = vec![1, 2, 3, 4, 5];
        let mut buffer = DataBuffer::from_vec(data);

        assert!(buffer.set_position(3).is_ok());
        assert_eq!(buffer.position(), 3);

        // Test setting position beyond buffer length
        assert!(buffer.set_position(10).is_err());
    }

    #[test]
    fn test_data_buffer_clear() {
        let mut buffer = DataBuffer::from_vec(vec![1, 2, 3, 4, 5]);
        buffer.read_byte().unwrap();
        assert_eq!(buffer.position(), 1);

        buffer.clear();
        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.position(), 0);
    }

    #[test]
    fn test_data_buffer_error_handling() {
        let mut buffer = DataBuffer::from_vec(vec![1, 2, 3]);

        // Read past end
        buffer.read_byte().unwrap(); // position = 1
        buffer.read_byte().unwrap(); // position = 2
        buffer.read_byte().unwrap(); // position = 3

        assert!(buffer.read_byte().is_err());
        assert!(buffer.read_slice(1).is_err());

        let mut buf = [0u8; 1];
        assert!(buffer.read_into(&mut buf).is_err());
    }
}