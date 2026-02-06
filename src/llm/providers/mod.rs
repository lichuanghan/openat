//! LLM providers implementations.

mod anthropic;
mod gemini;
mod groq;
mod minimax;
mod openai;
mod openrouter;

pub use anthropic::AnthropicProvider;
pub use gemini::GeminiProvider;
pub use groq::GroqProvider;
pub use minimax::MiniMaxProvider;
pub use openai::OpenAIProvider;
pub use openrouter::OpenRouterProvider;

use crate::config::Config;
use serde_json::Value;

/// Trait for LLM providers
#[async_trait::async_trait]
pub trait LLMProvider: Send + Sync {
    /// Send a chat request
    async fn chat(
        &self,
        messages: &[Value],
        model: &str,
        tools: &[Value],
    ) -> Result<crate::types::LLMResponse, String>;

    /// Provider name
    fn name(&self) -> &str;

    /// API base URL
    fn api_base(&self) -> &str;
}

/// Create a provider based on configuration priority
pub fn create_provider(config: &Config) -> Box<dyn LLMProvider> {
    // Priority: OpenRouter > Anthropic > OpenAI > Groq > Gemini > MiniMax
    if !config.providers.openrouter.api_key.is_empty() {
        Box::new(OpenRouterProvider::new(config.providers.openrouter.api_key.clone()))
    } else if !config.providers.anthropic.api_key.is_empty() {
        Box::new(AnthropicProvider::new(config.providers.anthropic.api_key.clone()))
    } else if !config.providers.openai.api_key.is_empty() {
        Box::new(OpenAIProvider::new(
            config.providers.openai.api_key.clone(),
            config.providers.openai.api_base.clone(),
        ))
    } else if !config.providers.groq.api_key.is_empty() {
        Box::new(GroqProvider::new(config.providers.groq.api_key.clone()))
    } else if !config.providers.gemini.api_key.is_empty() {
        Box::new(GeminiProvider::new(config.providers.gemini.api_key.clone()))
    } else if !config.providers.minimax.api_key.is_empty() {
        Box::new(MiniMaxProvider::new(config.providers.minimax.api_key.clone()))
    } else {
        Box::new(OpenRouterProvider::new(String::new()))
    }
}
