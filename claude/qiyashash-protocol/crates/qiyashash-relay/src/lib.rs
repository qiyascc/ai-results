//! # QiyasHash Relay
//!
//! Relay service for offline message delivery. Messages are split across
//! multiple relay nodes to prevent any single node from having complete
//! message data.
//!
//! ## Features
//!
//! - **Multi-Relay Distribution**: Messages split across 3-5 relays
//! - **Time-Based Expiry**: Automatic cleanup of old messages
//! - **No Metadata**: Relays cannot identify sender/recipient
//! - **Encrypted Blobs**: Only encrypted data stored

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]

pub mod client;
pub mod config;
pub mod error;
pub mod server;
pub mod storage;

pub use config::RelayConfig;
pub use error::{RelayError, Result};

/// Default number of relay nodes for message distribution
pub const DEFAULT_RELAY_COUNT: usize = 5;

/// Default message expiry in seconds (30 days)
pub const DEFAULT_MESSAGE_EXPIRY_SECS: u64 = 30 * 24 * 3600;

/// Maximum blob size (1 MB)
pub const MAX_BLOB_SIZE: usize = 1024 * 1024;
