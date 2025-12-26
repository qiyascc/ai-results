//! Error types for cryptographic operations

use thiserror::Error;

/// Result type alias for cryptographic operations
pub type Result<T> = std::result::Result<T, CryptoError>;

/// Errors that can occur during cryptographic operations
#[derive(Debug, Error)]
pub enum CryptoError {
    /// Key derivation failed
    #[error("Key derivation failed: {0}")]
    KeyDerivation(String),

    /// Invalid key length
    #[error("Invalid key length: expected {expected}, got {actual}")]
    InvalidKeyLength { expected: usize, actual: usize },

    /// Invalid signature
    #[error("Invalid signature")]
    InvalidSignature,

    /// Encryption failed
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    /// Decryption failed
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    /// Authentication failed (MAC verification)
    #[error("Message authentication failed")]
    AuthenticationFailed,

    /// Invalid public key
    #[error("Invalid public key: {0}")]
    InvalidPublicKey(String),

    /// Invalid chain state
    #[error("Invalid chain state: {0}")]
    InvalidChainState(String),

    /// Chain too long, re-keying required
    #[error("Chain length exceeded maximum ({max}), re-keying required")]
    ChainTooLong { max: u32 },

    /// Message too large
    #[error("Message too large: {size} bytes exceeds maximum {max}")]
    MessageTooLarge { size: usize, max: usize },

    /// Replay attack detected
    #[error("Replay attack detected: message {message_id} already processed")]
    ReplayDetected { message_id: u64 },

    /// Out of order message with too large gap
    #[error("Message gap too large: {gap} messages skipped")]
    MessageGapTooLarge { gap: u32 },

    /// Ratchet state corrupted
    #[error("Ratchet state corrupted: {0}")]
    RatchetCorrupted(String),

    /// Invalid protocol version
    #[error("Invalid protocol version: {0}")]
    InvalidVersion(u32),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Random number generation failed
    #[error("Random number generation failed")]
    RngFailed,

    /// Key exchange failed
    #[error("Key exchange failed: {0}")]
    KeyExchangeFailed(String),

    /// Identity verification failed
    #[error("Identity verification failed: {0}")]
    IdentityVerificationFailed(String),

    /// Prekey not found
    #[error("Prekey not found: {0}")]
    PrekeyNotFound(String),

    /// Session not established
    #[error("Session not established")]
    SessionNotEstablished,
}

impl From<bincode::Error> for CryptoError {
    fn from(err: bincode::Error) -> Self {
        CryptoError::Serialization(err.to_string())
    }
}

impl From<ed25519_dalek::SignatureError> for CryptoError {
    fn from(_: ed25519_dalek::SignatureError) -> Self {
        CryptoError::InvalidSignature
    }
}

impl From<aes_gcm::Error> for CryptoError {
    fn from(_: aes_gcm::Error) -> Self {
        CryptoError::DecryptionFailed("AEAD operation failed".to_string())
    }
}

impl From<chacha20poly1305::Error> for CryptoError {
    fn from(_: chacha20poly1305::Error) -> Self {
        CryptoError::DecryptionFailed("ChaCha20-Poly1305 operation failed".to_string())
    }
}
