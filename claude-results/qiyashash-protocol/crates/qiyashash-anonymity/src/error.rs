//! Error types for anonymity layer

use thiserror::Error;

/// Result type alias
pub type Result<T> = std::result::Result<T, AnonymityError>;

/// Anonymity layer errors
#[derive(Debug, Error)]
pub enum AnonymityError {
    /// Tor not available
    #[error("Tor is not available: {0}")]
    TorUnavailable(String),

    /// I2P not available
    #[error("I2P is not available: {0}")]
    I2PUnavailable(String),

    /// Connection failed
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// Circuit creation failed
    #[error("Circuit creation failed: {0}")]
    CircuitFailed(String),

    /// Transport error
    #[error("Transport error: {0}")]
    Transport(String),

    /// Timeout
    #[error("Operation timed out")]
    Timeout,

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Not initialized
    #[error("Anonymity layer not initialized")]
    NotInitialized,

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}
