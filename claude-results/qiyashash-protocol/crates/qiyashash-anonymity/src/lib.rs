//! # QiyasHash Anonymity
//!
//! Traffic obfuscation and anonymity layer for QiyasHash messaging.
//! Supports Tor and I2P networks for maximum privacy.
//!
//! ## Features
//!
//! - **Tor Integration**: Route traffic through Tor network
//! - **I2P Integration**: Use I2P garlic routing
//! - **Traffic Obfuscation**: Add noise and timing randomization
//! - **Cover Traffic**: Generate decoy messages to prevent traffic analysis

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]

pub mod config;
pub mod error;
pub mod obfuscation;
pub mod transport;

#[cfg(feature = "tor")]
pub mod tor;

#[cfg(feature = "i2p")]
pub mod i2p;

pub use config::AnonymityConfig;
pub use error::{AnonymityError, Result};
pub use obfuscation::TrafficObfuscator;
pub use transport::{AnonymousTransport, TransportType};
