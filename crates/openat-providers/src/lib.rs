//! LLM providers for openat
//!
//! Supports multiple LLM backends through a unified interface.

pub mod prelude {
    pub use super::{LLMProvider, LLMResponse, StreamResponse};
    pub use async_trait::async_trait;
    pub use serde_json::Value;
}

pub mod openai_compat;
pub mod anthropic;
pub mod openrouter;
pub mod groq;
pub mod gemini;
pub mod minimax;
pub mod ollama;

pub use openai_compat::{OpenAICompatConfig, OpenAICompatProvider};
pub use anthropic::AnthropicProvider;
pub use openrouter::OpenRouterProvider;
pub use groq::GroqProvider;
pub use gemini::GeminiProvider;
pub use minimax::MiniMaxProvider;
pub use ollama::OllamaProvider;

use async_trait::async_trait;
use futures_util::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Trait for LLM providers
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Send a chat request
    async fn chat(
        &self,
        messages: &[Value],
        model: &str,
        tools: &[Value],
    ) -> Result<LLMResponse, String>;

    /// Stream a chat response (optional - default returns error)
    fn stream(
        &self,
        _messages: &[Value],
        _model: &str,
        _tools: &[Value],
    ) -> Pin<Box<dyn Stream<Item = Result<StreamResponse, String>> + Send>> {
        Box::pin(futures_util::stream::iter(vec![Err(
            "Streaming not supported by this provider".to_string(),
        )]))
    }

    /// Whether this provider supports streaming
    fn supports_streaming(&self) -> bool {
        false
    }

    /// Provider name
    fn name(&self) -> &str;

    /// API base URL
    fn api_base(&self) -> &str;
}

/// LLM response
#[derive(Debug, Clone)]
pub struct LLMResponse {
    pub content: Option<String>,
    pub tool_calls: Vec<openai_compat::ToolCall>,
    pub finish_reason: String,
}

impl LLMResponse {
    pub fn is_empty(&self) -> bool {
        self.content.as_ref().map(|s| s.is_empty()).unwrap_or(true)
            && self.tool_calls.is_empty()
    }
}

/// Streaming response chunk
#[derive(Debug, Clone)]
pub struct StreamResponse {
    pub content: String,
    pub is_final: bool,
    pub tool_calls: Vec<openai_compat::ToolCall>,
}
