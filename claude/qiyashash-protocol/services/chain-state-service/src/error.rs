//! Error types for the Chain State Service

use actix_web::{HttpResponse, ResponseError};
use std::fmt;

/// Service-specific errors
#[derive(Debug)]
pub enum ChainStateError {
    /// Chain not found
    ChainNotFound(String),
    /// Invalid chain state
    InvalidState(String),
    /// Hash mismatch
    HashMismatch { expected: String, actual: String },
    /// Storage error
    StorageError(String),
    /// Serialization error
    SerializationError(String),
    /// Validation error
    ValidationError(String),
    /// Internal error
    InternalError(String),
}

impl fmt::Display for ChainStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ChainNotFound(id) => write!(f, "Chain not found: {}", id),
            Self::InvalidState(msg) => write!(f, "Invalid chain state: {}", msg),
            Self::HashMismatch { expected, actual } => {
                write!(f, "Hash mismatch: expected {}, got {}", expected, actual)
            }
            Self::StorageError(msg) => write!(f, "Storage error: {}", msg),
            Self::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            Self::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            Self::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for ChainStateError {}

impl ResponseError for ChainStateError {
    fn error_response(&self) -> HttpResponse {
        match self {
            Self::ChainNotFound(_) => HttpResponse::NotFound().json(serde_json::json!({
                "error": "not_found",
                "message": self.to_string()
            })),
            Self::InvalidState(_) | Self::ValidationError(_) => {
                HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "bad_request",
                    "message": self.to_string()
                }))
            }
            Self::HashMismatch { .. } => HttpResponse::Conflict().json(serde_json::json!({
                "error": "hash_mismatch",
                "message": self.to_string()
            })),
            _ => HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "internal_error",
                "message": "An internal error occurred"
            })),
        }
    }
}
