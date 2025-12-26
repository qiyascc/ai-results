//! # QiyasHash DHT
//!
//! Distributed Hash Table implementation for decentralized message storage
//! and retrieval. Based on Kademlia protocol via libp2p.
//!
//! ## Features
//!
//! - **Decentralized Storage**: Messages stored across multiple peers
//! - **Reed-Solomon Encoding**: Message fragmentation with erasure coding
//! - **Automatic Expiry**: Time-based message expiration
//! - **Anonymity**: No metadata leakage about sender/recipient
//!
//! ## Architecture
//!
//! Messages are:
//! 1. Encrypted by the sender
//! 2. Split into fragments using Reed-Solomon
//! 3. Distributed across multiple DHT nodes
//! 4. Retrieved and reconstructed by recipient

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]

pub mod config;
pub mod error;
pub mod fragment;
pub mod node;
pub mod storage;

pub use config::DhtConfig;
pub use error::{DhtError, Result};
pub use fragment::{Fragment, FragmentId, MessageFragments};
pub use node::{DhtNode, DhtEvent};
pub use storage::DhtStorage;

/// Default fragment count for Reed-Solomon encoding
pub const DEFAULT_FRAGMENT_COUNT: usize = 5;

/// Default fragment threshold (minimum fragments needed for reconstruction)
pub const DEFAULT_FRAGMENT_THRESHOLD: usize = 3;

/// Default message expiry in seconds (30 days)
pub const DEFAULT_MESSAGE_EXPIRY_SECS: u64 = 30 * 24 * 3600;

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::config::DhtConfig;
    pub use crate::error::{DhtError, Result};
    pub use crate::fragment::{Fragment, FragmentId, MessageFragments};
    pub use crate::node::{DhtNode, DhtEvent};
    pub use crate::storage::DhtStorage;
}
