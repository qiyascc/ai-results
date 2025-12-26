//! Error types for Relay Coordination Service

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoordinationError {
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Registration failed: {0}")]
    RegistrationFailed(String),

    #[error("No available relays")]
    NoAvailableRelays,

    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}
