//! Error types for QiyasHash core

use thiserror::Error;

/// Result type alias
pub type Result<T> = std::result::Result<T, Error>;

/// Core error types
#[derive(Debug, Error)]
pub enum Error {
    /// Cryptographic operation failed
    #[error("Crypto error: {0}")]
    Crypto(#[from] qiyashash_crypto::CryptoError),

    /// Message not found
    #[error("Message not found: {0}")]
    MessageNotFound(String),

    /// Session not found
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// User not found
    #[error("User not found: {0}")]
    UserNotFound(String),

    /// Session not established
    #[error("Session not established with user {0}")]
    SessionNotEstablished(String),

    /// Invalid message format
    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    /// Message too large
    #[error("Message too large: {size} bytes exceeds maximum {max}")]
    MessageTooLarge { size: usize, max: usize },

    /// Attachment too large
    #[error("Attachment too large: {size} bytes exceeds maximum {max}")]
    AttachmentTooLarge { size: usize, max: usize },

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// Invalid state
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Timeout
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Rate limited
    #[error("Rate limited: retry after {retry_after_secs} seconds")]
    RateLimited { retry_after_secs: u64 },

    /// Protocol version mismatch
    #[error("Protocol version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u32, actual: u32 },

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

impl From<bincode::Error> for Error {
    fn from(err: bincode::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Storage(err.to_string())
    }
}
