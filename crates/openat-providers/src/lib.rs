//! LLM providers for openat
//!
//! Supports multiple LLM backends through a unified interface.

pub mod prelude {
    pub use super::{LLMProvider, LLMResponse};
    pub use async_trait::async_trait;
    pub use serde_json::Value;
}

pub mod openai_compat;
pub mod anthropic;
pub mod openrouter;
pub mod groq;
pub mod gemini;
pub mod minimax;

pub use openai_compat::{OpenAICompatConfig, OpenAICompatProvider};
pub use anthropic::AnthropicProvider;
pub use openrouter::OpenRouterProvider;
pub use groq::GroqProvider;
pub use gemini::GeminiProvider;
pub use minimax::MiniMaxProvider;

use async_trait::async_trait;
use serde_json::Value;

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
