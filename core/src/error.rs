//! Error types for SteelCtl

use thiserror::Error;

/// Result type alias for SteelCtl operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during device operations
#[derive(Error, Debug)]
pub enum Error {
    /// HID communication error
    #[error("HID error: {0}")]
    Hid(#[from] hidapi::HidError),

    /// Device not found
    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    /// Invalid parameter value
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Communication timeout
    #[error("Device communication timeout")]
    Timeout,

    /// Unsupported operation
    #[error("Unsupported operation: {0}")]
    Unsupported(String),
}
