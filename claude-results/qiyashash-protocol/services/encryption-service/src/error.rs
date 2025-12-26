//! Error types for encryption service

use actix_web::{HttpResponse, ResponseError};
use std::fmt;

/// Service error type
#[derive(Debug)]
pub enum ServiceError {
    /// Crypto error
    Crypto(String),
    /// Session not found
    SessionNotFound(String),
    /// Invalid request
    InvalidRequest(String),
    /// Storage error
    Storage(String),
    /// Internal error
    Internal(String),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Crypto(msg) => write!(f, "Crypto error: {}", msg),
            Self::SessionNotFound(id) => write!(f, "Session not found: {}", id),
            Self::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            Self::Storage(msg) => write!(f, "Storage error: {}", msg),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl ResponseError for ServiceError {
    fn error_response(&self) -> HttpResponse {
        match self {
            Self::Crypto(msg) => HttpResponse::BadRequest().json(ErrorResponse {
                error: "crypto_error".to_string(),
                message: msg.clone(),
            }),
            Self::SessionNotFound(id) => HttpResponse::NotFound().json(ErrorResponse {
                error: "session_not_found".to_string(),
                message: format!("Session {} not found", id),
            }),
            Self::InvalidRequest(msg) => HttpResponse::BadRequest().json(ErrorResponse {
                error: "invalid_request".to_string(),
                message: msg.clone(),
            }),
            Self::Storage(msg) => HttpResponse::InternalServerError().json(ErrorResponse {
                error: "storage_error".to_string(),
                message: msg.clone(),
            }),
            Self::Internal(msg) => HttpResponse::InternalServerError().json(ErrorResponse {
                error: "internal_error".to_string(),
                message: msg.clone(),
            }),
        }
    }
}

/// Error response body
#[derive(serde::Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

impl From<qiyashash_crypto::CryptoError> for ServiceError {
    fn from(err: qiyashash_crypto::CryptoError) -> Self {
        ServiceError::Crypto(err.to_string())
    }
}

impl From<std::io::Error> for ServiceError {
    fn from(err: std::io::Error) -> Self {
        ServiceError::Storage(err.to_string())
    }
}
