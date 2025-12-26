//! Error types for Metadata Nullification Service

use thiserror::Error;

#[derive(Error, Debug)]
pub enum NullificationError {
    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Processing error: {0}")]
    ProcessingError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}
