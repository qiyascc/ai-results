//! Error types for Identity Service

use actix_web::{HttpResponse, ResponseError};
use serde::Serialize;
use std::fmt;

/// Service error types
#[derive(Debug)]
pub enum ServiceError {
    /// User not found
    NotFound(String),
    /// Invalid request
    BadRequest(String),
    /// Storage error
    Storage(String),
    /// Crypto error
    Crypto(String),
    /// Verification failed
    VerificationFailed(String),
    /// Internal error
    Internal(String),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ServiceError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            ServiceError::Storage(msg) => write!(f, "Storage error: {}", msg),
            ServiceError::Crypto(msg) => write!(f, "Crypto error: {}", msg),
            ServiceError::VerificationFailed(msg) => write!(f, "Verification failed: {}", msg),
            ServiceError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

/// Error response body
#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    code: String,
}

impl ResponseError for ServiceError {
    fn error_response(&self) -> HttpResponse {
        let (status, code) = match self {
            ServiceError::NotFound(_) => (actix_web::http::StatusCode::NOT_FOUND, "NOT_FOUND"),
            ServiceError::BadRequest(_) => {
                (actix_web::http::StatusCode::BAD_REQUEST, "BAD_REQUEST")
            }
            ServiceError::Storage(_) => (
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                "STORAGE_ERROR",
            ),
            ServiceError::Crypto(_) => (
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                "CRYPTO_ERROR",
            ),
            ServiceError::VerificationFailed(_) => {
                (actix_web::http::StatusCode::UNAUTHORIZED, "VERIFICATION_FAILED")
            }
            ServiceError::Internal(_) => (
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
            ),
        };

        HttpResponse::build(status).json(ErrorResponse {
            error: self.to_string(),
            code: code.to_string(),
        })
    }
}

impl From<rocksdb::Error> for ServiceError {
    fn from(err: rocksdb::Error) -> Self {
        ServiceError::Storage(err.to_string())
    }
}

impl From<qiyashash_crypto::CryptoError> for ServiceError {
    fn from(err: qiyashash_crypto::CryptoError) -> Self {
        ServiceError::Crypto(err.to_string())
    }
}

impl From<serde_json::Error> for ServiceError {
    fn from(err: serde_json::Error) -> Self {
        ServiceError::Internal(err.to_string())
    }
}

impl From<hex::FromHexError> for ServiceError {
    fn from(err: hex::FromHexError) -> Self {
        ServiceError::BadRequest(err.to_string())
    }
}
