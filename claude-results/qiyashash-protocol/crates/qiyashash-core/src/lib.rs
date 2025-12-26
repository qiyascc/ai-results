//! # QiyasHash Core
//!
//! Core types, traits, and utilities for the QiyasHash E2E messaging protocol.
//!
//! This crate provides:
//! - Message types and serialization
//! - User and session identifiers
//! - Storage traits
//! - Common error types

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]

pub mod error;
pub mod message;
pub mod session;
pub mod storage;
pub mod types;
pub mod user;

pub use error::{Error, Result};
pub use message::{Message, MessageEnvelope, MessageId, MessageStatus};
pub use session::{Session, SessionId, SessionState};
pub use types::{DeviceId, Timestamp, UserId};
pub use user::{User, UserProfile};

/// Protocol version
pub const PROTOCOL_VERSION: u32 = 1;

/// Maximum message size (64 KB)
pub const MAX_MESSAGE_SIZE: usize = 65536;

/// Maximum attachment size (25 MB)
pub const MAX_ATTACHMENT_SIZE: usize = 25 * 1024 * 1024;

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::error::{Error, Result};
    pub use crate::message::{Message, MessageEnvelope, MessageId};
    pub use crate::session::{Session, SessionId};
    pub use crate::storage::{MessageStore, SessionStore, UserStore};
    pub use crate::types::{DeviceId, Timestamp, UserId};
    pub use crate::user::{User, UserProfile};
}
