//! # QiyasHash Cryptographic Library
//!
//! This crate provides the core cryptographic primitives for the QiyasHash
//! end-to-end encrypted messaging protocol.
//!
//! ## Security Features
//!
//! - **Forward Secrecy**: Each message uses ephemeral keys, compromised long-term
//!   keys cannot decrypt past messages
//! - **Backward Secrecy**: Key ratcheting ensures future messages remain secure
//!   after key compromise
//! - **Deniability**: Uses symmetric authentication (HMAC) rather than signatures
//!   for message authentication
//! - **Metadata Protection**: All cryptographic operations are constant-time
//!
//! ## Core Components
//!
//! - [`identity`]: Identity key management and X3DH key agreement
//! - [`ratchet`]: Double Ratchet algorithm for message encryption
//! - [`keys`]: Key types and derivation functions
//! - [`aead`]: Authenticated encryption (ChaCha20-Poly1305, AES-256-GCM)
//! - [`chain`]: Chain state management for message ordering

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]

pub mod aead;
pub mod chain;
pub mod error;
pub mod identity;
pub mod keys;
pub mod kdf;
pub mod ratchet;
pub mod x3dh;

pub use error::{CryptoError, Result};

/// Protocol version for compatibility checking
pub const PROTOCOL_VERSION: u32 = 1;

/// Maximum message size in bytes (64 KB)
pub const MAX_MESSAGE_SIZE: usize = 65536;

/// Maximum chain length before forced re-keying
pub const MAX_CHAIN_LENGTH: u32 = 1000;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::aead::{Aead, AeadKey, Nonce};
    pub use crate::chain::{ChainKey, ChainState, MessageKey};
    pub use crate::error::{CryptoError, Result};
    pub use crate::identity::{Identity, IdentityKeyPair, IdentityPublicKey};
    pub use crate::keys::{EphemeralKeyPair, PreKeyBundle, SignedPreKey};
    pub use crate::ratchet::{DoubleRatchet, RatchetHeader, RatchetState};
    pub use crate::x3dh::{X3DHKeyAgreement, X3DHSharedSecret};
}
