//! Error types for DHT Peer Service

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DhtError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Record not found: {0}")]
    RecordNotFound(String),

    #[error("Invalid peer ID: {0}")]
    InvalidPeerId(String),

    #[error("Bootstrap failed: {0}")]
    BootstrapFailed(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}
