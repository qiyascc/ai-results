//! # QiyasHash Desktop Client
//!
//! Desktop client library for QiyasHash messaging.
//! Designed for use with Tauri for the UI layer.
//!
//! ## Features
//!
//! - Complete messaging functionality
//! - Local storage with encryption
//! - Multi-device support
//! - Notification handling

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]

pub mod app;
pub mod commands;
pub mod state;
pub mod storage;

pub use app::App;
pub use state::AppState;

/// Application version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::app::App;
    pub use crate::commands::*;
    pub use crate::state::AppState;
}
