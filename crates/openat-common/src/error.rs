//! Common error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CommonError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    Permission(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Unknown: {0}")]
    Unknown(String),
}

pub type CommonResult<T> = Result<T, CommonError>;
