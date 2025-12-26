//! DHT error types

use thiserror::Error;

/// Result type alias
pub type Result<T> = std::result::Result<T, DhtError>;

/// DHT errors
#[derive(Debug, Error)]
pub enum DhtError {
    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Fragment not found
    #[error("Fragment not found: {0}")]
    FragmentNotFound(String),

    /// Message not found
    #[error("Message not found: {0}")]
    MessageNotFound(String),

    /// Reconstruction failed
    #[error("Message reconstruction failed: need {needed} fragments, have {have}")]
    ReconstructionFailed { needed: usize, have: usize },

    /// Encoding error
    #[error("Encoding error: {0}")]
    EncodingError(String),

    /// Decoding error
    #[error("Decoding error: {0}")]
    DecodingError(String),

    /// Node not connected
    #[error("Node not connected to DHT network")]
    NotConnected,

    /// Peer not found
    #[error("Peer not found: {0}")]
    PeerNotFound(String),

    /// Timeout
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Invalid fragment
    #[error("Invalid fragment: {0}")]
    InvalidFragment(String),

    /// Message expired
    #[error("Message expired")]
    MessageExpired,

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<std::io::Error> for DhtError {
    fn from(err: std::io::Error) -> Self {
        DhtError::Storage(err.to_string())
    }
}

impl From<bincode::Error> for DhtError {
    fn from(err: bincode::Error) -> Self {
        DhtError::EncodingError(err.to_string())
    }
}

impl From<sled::Error> for DhtError {
    fn from(err: sled::Error) -> Self {
        DhtError::Storage(err.to_string())
    }
}
