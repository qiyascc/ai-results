//! Protocol error types

use thiserror::Error;

/// Result type alias
pub type Result<T> = std::result::Result<T, ProtocolError>;

/// Protocol errors
#[derive(Debug, Error)]
pub enum ProtocolError {
    /// Core error
    #[error("Core error: {0}")]
    Core(#[from] qiyashash_core::Error),

    /// Crypto error
    #[error("Crypto error: {0}")]
    Crypto(#[from] qiyashash_crypto::CryptoError),

    /// Session not found
    #[error("Session not found for user {0}")]
    SessionNotFound(String),

    /// Session not established
    #[error("Session not established with user {0}")]
    SessionNotEstablished(String),

    /// Invalid prekey bundle
    #[error("Invalid prekey bundle: {0}")]
    InvalidPreKeyBundle(String),

    /// Key exchange failed
    #[error("Key exchange failed: {0}")]
    KeyExchangeFailed(String),

    /// Invalid message format
    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    /// Decryption failed
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    /// Identity mismatch
    #[error("Identity mismatch: expected {expected}, got {actual}")]
    IdentityMismatch { expected: String, actual: String },

    /// Untrusted identity
    #[error("Untrusted identity for user {0}")]
    UntrustedIdentity(String),

    /// Protocol version mismatch
    #[error("Protocol version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u32, actual: u32 },

    /// Chain verification failed
    #[error("Chain verification failed: {0}")]
    ChainVerificationFailed(String),

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Not initialized
    #[error("Protocol client not initialized")]
    NotInitialized,

    /// Already initialized
    #[error("Protocol client already initialized")]
    AlreadyInitialized,

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}
