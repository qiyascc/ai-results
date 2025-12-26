//! # QiyasHash Protocol
//!
//! High-level messaging protocol implementation combining X3DH key exchange,
//! Double Ratchet encryption, and chain state management.
//!
//! ## Overview
//!
//! This crate provides the complete protocol stack for secure messaging:
//!
//! - **Session Management**: Establish and maintain encrypted sessions
//! - **Message Encryption**: Encrypt/decrypt messages with forward secrecy
//! - **Chain State**: Track message ordering and integrity
//! - **Protocol Messages**: Handle all protocol-level operations
//!
//! ## Usage
//!
//! ```ignore
//! use qiyashash_protocol::{ProtocolClient, ClientConfig};
//!
//! // Create client
//! let config = ClientConfig::default();
//! let client = ProtocolClient::new(config, storage).await?;
//!
//! // Send message
//! let envelope = client.encrypt_message(&recipient, message).await?;
//!
//! // Receive message
//! let message = client.decrypt_message(&envelope).await?;
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]

pub mod client;
pub mod config;
pub mod error;
pub mod handlers;
pub mod protocol;
pub mod session_manager;

pub use client::ProtocolClient;
pub use config::ClientConfig;
pub use error::{ProtocolError, Result};
pub use protocol::{ProtocolMessage, ProtocolMessageType};
pub use session_manager::SessionManager;

/// Protocol version
pub const PROTOCOL_VERSION: u32 = 1;

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::client::ProtocolClient;
    pub use crate::config::ClientConfig;
    pub use crate::error::{ProtocolError, Result};
    pub use crate::protocol::{ProtocolMessage, ProtocolMessageType};
    pub use crate::session_manager::SessionManager;
}
