//! Relay error types

use thiserror::Error;

/// Result type alias
pub type Result<T> = std::result::Result<T, RelayError>;

/// Relay errors
#[derive(Debug, Error)]
pub enum RelayError {
    /// Not connected
    #[error("Not connected to relay")]
    NotConnected,

    /// Connection failed
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// Blob not found
    #[error("Blob not found: {0}")]
    BlobNotFound(String),

    /// Blob too large
    #[error("Blob too large: {size} bytes exceeds maximum {max}")]
    BlobTooLarge { size: usize, max: usize },

    /// Blob expired
    #[error("Blob expired")]
    BlobExpired,

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Invalid blob
    #[error("Invalid blob: {0}")]
    InvalidBlob(String),

    /// Timeout
    #[error("Operation timed out")]
    Timeout,

    /// Rate limited
    #[error("Rate limited: retry after {retry_after_secs} seconds")]
    RateLimited { retry_after_secs: u64 },

    /// Not enough relays
    #[error("Not enough relays available: have {have}, need {need}")]
    NotEnoughRelays { have: usize, need: usize },

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// TLS error
    #[error("TLS error: {0}")]
    Tls(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<std::io::Error> for RelayError {
    fn from(err: std::io::Error) -> Self {
        RelayError::Network(err.to_string())
    }
}
