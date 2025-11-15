//! Error handling for Draco operations
//!
//! This module provides error handling utilities that mirror the C++ Draco Status class
//! while leveraging Rust's Result type and error handling ecosystem.

use std::fmt;

/// Draco error codes
///
/// These correspond to the Status::Code enum in the C++ implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    /// Operation completed successfully
    Ok = 0,
    /// Used for general errors
    DracoError = -1,
    /// Error when handling input or output stream
    IoError = -2,
    /// Invalid parameter passed to a function
    InvalidParameter = -3,
    /// Input not compatible with the current version
    UnsupportedVersion = -4,
    /// Input was created with an unknown version of the library
    UnknownVersion = -5,
    /// Input contains feature that is not supported
    UnsupportedFeature = -6,
}

impl ErrorCode {
    /// Returns the name of this error code as a string
    pub const fn name(self) -> &'static str {
        match self {
            ErrorCode::Ok => "OK",
            ErrorCode::DracoError => "DRACO_ERROR",
            ErrorCode::IoError => "IO_ERROR",
            ErrorCode::InvalidParameter => "INVALID_PARAMETER",
            ErrorCode::UnsupportedVersion => "UNSUPPORTED_VERSION",
            ErrorCode::UnknownVersion => "UNKNOWN_VERSION",
            ErrorCode::UnsupportedFeature => "UNSUPPORTED_FEATURE",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Default for ErrorCode {
    fn default() -> Self {
        ErrorCode::Ok
    }
}

/// Draco error type
///
/// This represents an error that can occur during Draco operations, equivalent to
/// the C++ Status class but using Rust's error handling conventions.
#[derive(Debug, Clone)]
pub struct DracoError {
    code: ErrorCode,
    message: String,
}

impl DracoError {
    /// Creates a new DracoError with the given code and message
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Creates a new DracoError with the given code only
    pub fn from_code(code: ErrorCode) -> Self {
        Self {
            code,
            message: code.name().to_string(),
        }
    }

    /// Returns the error code
    pub fn code(&self) -> ErrorCode {
        self.code
    }

    /// Returns the error message
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns true if this error represents success (ErrorCode::Ok)
    pub fn is_ok(&self) -> bool {
        self.code == ErrorCode::Ok
    }

    /// Returns true if this error represents a failure (anything other than ErrorCode::Ok)
    pub fn is_error(&self) -> bool {
        self.code != ErrorCode::Ok
    }
}

impl fmt::Display for DracoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for DracoError {}

/// Convenient constructors for common error types
impl DracoError {
    /// Creates a general Draco error
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::DracoError, message)
    }

    /// Creates an I/O error
    pub fn io_error(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::IoError, message)
    }

    /// Creates an invalid parameter error
    pub fn invalid_parameter(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidParameter, message)
    }

    /// Creates an unsupported version error
    pub fn unsupported_version(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::UnsupportedVersion, message)
    }

    /// Creates an unknown version error
    pub fn unknown_version(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::UnknownVersion, message)
    }

    /// Creates an unsupported feature error
    pub fn unsupported_feature(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::UnsupportedFeature, message)
    }
}

/// Type alias for Result with DracoError
///
/// This is the primary return type for Draco operations, equivalent to
/// StatusOr<T> in the C++ implementation.
pub type StatusResult<T> = Result<T, DracoError>;

/// Status type that mirrors the C++ Status class
///
/// While in idiomatic Rust we'd typically use Result<T, DracoError> directly,
/// this type is provided for compatibility with the C++ API and for
/// operations that don't return a value.
#[derive(Debug, Clone)]
pub struct Status {
    code: ErrorCode,
    message: String,
}

impl Status {
    /// Creates a new Status
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Returns the error code
    pub fn code(&self) -> ErrorCode {
        self.code
    }

    /// Returns the error message
    pub fn error_msg(&self) -> &str {
        &self.message
    }

    /// Returns true if this status represents success
    pub fn is_ok(&self) -> bool {
        self.code == ErrorCode::Ok
    }

    /// Converts this Status to a Result<()>
    pub fn into_result(self) -> StatusResult<()> {
        if self.is_ok() {
            Ok(())
        } else {
            Err(DracoError::new(self.code, self.message))
        }
    }

    /// Converts this Status to a Result<T>
    pub fn into_result_with<T>(self, value: T) -> StatusResult<T> {
        if self.is_ok() {
            Ok(value)
        } else {
            Err(DracoError::new(self.code, self.message))
        }
    }

    /// Returns a success status
    pub fn ok() -> Self {
        Self {
            code: ErrorCode::Ok,
            message: "OK".to_string(),
        }
    }

    /// Creates an error status
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::DracoError, message)
    }

    /// Returns a string representation of the status code
    pub fn code_string(&self) -> String {
        self.code.name().to_string()
    }

    /// Returns a string with both code and message
    pub fn code_and_error_string(&self) -> String {
        if self.is_ok() {
            "OK".to_string()
        } else {
            format!("{}: {}", self.code.name(), self.message)
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code_and_error_string())
    }
}

impl Default for Status {
    fn default() -> Self {
        Self::ok()
    }
}

impl From<DracoError> for Status {
    fn from(err: DracoError) -> Self {
        Self::new(err.code(), err.message())
    }
}

impl From<std::io::Error> for Status {
    fn from(err: std::io::Error) -> Self {
        Self::new(ErrorCode::IoError, err.to_string())
    }
}

impl From<std::io::Error> for DracoError {
    fn from(err: std::io::Error) -> Self {
        Self::io_error(err.to_string())
    }
}

/// Creates a success status
///
/// Equivalent to OkStatus() in the C++ code
pub fn ok_status() -> Status {
    Status::ok()
}

/// Creates an error status with the given message
///
/// Equivalent to ErrorStatus() in the C++ code
pub fn error_status(message: impl Into<String>) -> Status {
    Status::error(message)
}

/// Creates an I/O error status
pub fn io_error_status(message: impl Into<String>) -> Status {
    Status::new(ErrorCode::IoError, message)
}

/// Macro for early return on error, similar to DRACO_RETURN_IF_ERROR in C++
///
/// This macro evaluates an expression that returns a StatusResult<T>. If the result
/// is an error, it returns the error immediately.
#[macro_export]
macro_rules! return_if_error {
    ($expr:expr) => {
        match $expr {
            Ok(value) => value,
            Err(error) => return Err(error),
        }
    };
}

/// Macro for early return on error with status conversion
#[macro_export]
macro_rules! return_if_error_status {
    ($expr:expr) => {
        match $expr {
            Ok(value) => value,
            Err(error) => return Err(Status::from(error)),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(ErrorCode::Ok.name(), "OK");
        assert_eq!(ErrorCode::DracoError.name(), "DRACO_ERROR");
        assert_eq!(ErrorCode::IoError.name(), "IO_ERROR");
    }

    #[test]
    fn test_draco_error() {
        let error = DracoError::error("test message");
        assert_eq!(error.code(), ErrorCode::DracoError);
        assert_eq!(error.message(), "test message");
        assert!(error.is_error());
        assert!(!error.is_ok());
    }

    #[test]
    fn test_status() {
        let status = Status::ok();
        assert!(status.is_ok());
        assert_eq!(status.code(), ErrorCode::Ok);

        let status = Status::error("test error");
        assert!(!status.is_ok());
        assert_eq!(status.code(), ErrorCode::DracoError);
        assert_eq!(status.error_msg(), "test error");
    }

    #[test]
    fn test_status_to_result() {
        let ok_status = Status::ok();
        assert!(ok_status.into_result().is_ok());

        let err_status = Status::error("test error");
        assert!(err_status.into_result().is_err());
    }

    #[test]
    fn test_status_result_with_value() {
        let ok_status = Status::ok();
        let result: StatusResult<i32> = ok_status.into_result_with(42);
        assert_eq!(result.unwrap(), 42);

        let err_status = Status::error("test error");
        let result: StatusResult<i32> = err_status.into_result_with(42);
        assert!(result.is_err());
    }

    #[test]
    fn test_convenience_constructors() {
        let io_err = DracoError::io_error("file not found");
        assert_eq!(io_err.code(), ErrorCode::IoError);

        let param_err = DracoError::invalid_parameter("negative value");
        assert_eq!(param_err.code(), ErrorCode::InvalidParameter);
    }

    #[test]
    fn test_display() {
        let error = DracoError::error("test message");
        assert_eq!(format!("{}", error), "DRACO_ERROR: test message");

        let status = Status::ok();
        assert_eq!(format!("{}", status), "OK");

        let err_status = Status::error("test error");
        assert_eq!(format!("{}", err_status), "DRACO_ERROR: test error");
    }

    #[test]
    fn test_from_conversions() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let draco_error: DracoError = io_error.into();
        assert_eq!(draco_error.code(), ErrorCode::IoError);
        assert!(draco_error.message().contains("file not found"));
    }
}