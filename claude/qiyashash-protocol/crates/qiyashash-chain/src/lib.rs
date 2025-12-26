//! # QiyasHash Chain
//!
//! Chain state management for message ordering and integrity verification.
//! Provides cryptographic proofs of message order without revealing content.
//!
//! ## Features
//!
//! - **Order Verification**: Prove message sequence without content
//! - **Integrity Checking**: Detect tampering or missing messages
//! - **Role Swapping**: Support identity swaps within chains
//! - **Replay Protection**: Prevent message replay attacks

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]

// Re-export chain types from crypto crate
pub use qiyashash_crypto::chain::{
    ChainKey, ChainLink, ChainLinkType, ChainProof, ChainState,
    ChainVerifier, MessageKey, compute_message_hash,
};

pub mod manager;
pub mod storage;

pub use manager::ChainManager;
pub use storage::ChainStorage;
