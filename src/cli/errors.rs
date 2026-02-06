//! Unified error handling module using thiserror.
//!
//! # Error Categories
//!
//! - `ConfigError`: Configuration-related errors
//! - `LlmError`: LLM provider errors
//! - `ChannelError`: Messaging channel errors
//! - `ToolError`: Tool execution errors
//! - `SessionError`: Session management errors
//! - `SchedulerError`: Job scheduling errors
//! - `BusError`: Message bus errors

use thiserror::Error;

/// Unified error type for the application
#[derive(Error, Debug)]
pub enum AppError {
    /// Configuration errors
    #[error(transparent)]
    Config(#[from] ConfigError),

    /// LLM provider errors
    #[error(transparent)]
    Llm(#[from] LlmError),

    /// Channel errors
    #[error(transparent)]
    Channel(#[from] ChannelError),

    /// Tool execution errors
    #[error(transparent)]
    Tool(#[from] ToolError),

    /// Session errors
    #[error(transparent)]
    Session(#[from] SessionError),

    /// Scheduler errors
    #[error(transparent)]
    Scheduler(#[from] SchedulerError),

    /// Message bus errors
    #[error(transparent)]
    Bus(#[from] BusError),

    /// IO errors
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// JSON parsing errors
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// Network errors
    #[error(transparent)]
    Http(#[from] reqwest::Error),

    /// Parse errors
    #[error(transparent)]
    Parse(#[from] Box<dyn std::error::Error + Send + Sync>),

    /// Other errors
    #[error("{0}")]
    Other(String),
}

/// Configuration-related errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file not found: {0}")]
    NotFound(String),

    #[error("Failed to parse config: {0}")]
    Parse(String),

    #[error("Invalid config value: {0}")]
    Invalid(String),

    #[error("Missing required config: {0}")]
    Missing(String),

    #[error("Config validation failed: {0}")]
    Validation(String),
}

/// LLM provider errors
#[derive(Error, Debug)]
pub enum LlmError {
    #[error("Provider not configured: {0}")]
    NotConfigured(String),

    #[error("API request failed: {0}")]
    ApiError(String),

    #[error("Invalid response from API: {0}")]
    InvalidResponse(String),

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("Model not supported: {0}")]
    UnsupportedModel(String),

    #[error("Token limit exceeded")]
    TokenLimit,

    #[error("LLM error: {0}")]
    Other(String),
}

/// Messaging channel errors
#[derive(Error, Debug)]
pub enum ChannelError {
    #[error("Channel not enabled: {0}")]
    NotEnabled(String),

    #[error("Failed to connect to channel: {0}")]
    ConnectionFailed(String),

    #[error("Channel disconnected: {0}")]
    Disconnected(String),

    #[error("Message delivery failed: {0}")]
    DeliveryFailed(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Channel error: {0}")]
    Other(String),
}

/// Tool execution errors
#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Tool timeout: {0}")]
    Timeout(String),

    #[error("Invalid tool arguments: {0}")]
    InvalidArgs(String),

    #[error("Tool error: {0}")]
    Other(String),
}

/// Session management errors
#[derive(Error, Debug)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),

    #[error("Failed to load session: {0}")]
    LoadFailed(String),

    #[error("Failed to save session: {0}")]
    SaveFailed(String),

    #[error("Session corrupted: {0}")]
    Corrupted(String),

    #[error("Session error: {0}")]
    Other(String),
}

/// Job scheduling errors
#[derive(Error, Debug)]
pub enum SchedulerError {
    #[error("Job not found: {0}")]
    NotFound(String),

    #[error("Invalid cron expression: {0}")]
    InvalidCron(String),

    #[error("Scheduler error: {0}")]
    Other(String),
}

/// Message bus errors
#[derive(Error, Debug)]
pub enum BusError {
    #[error("Bus closed")]
    Closed,

    #[error("Send timeout")]
    SendTimeout,

    #[error("Bus error: {0}")]
    Other(String),
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, AppError>;

/// Helper to convert any error to AppError
impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Other(s)
    }
}

/// Helper to convert &str to AppError
impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::Other(s.to_string())
    }
}
