//! LLM providers implementations.

mod anthropic;
mod deepseek;
mod gemini;
mod groq;
mod litellm;
mod minimax;
mod moonshot;
mod openai;
mod openrouter;
mod transcription;
mod vllm;
mod zhipu;

pub use anthropic::AnthropicProvider;
pub use deepseek::DeepSeekProvider;
pub use gemini::GeminiProvider;
pub use groq::GroqProvider;
pub use litellm::LiteLLMProvider;
pub use minimax::MiniMaxProvider;
pub use moonshot::MoonshotProvider;
pub use openai::OpenAIProvider;
pub use openrouter::OpenRouterProvider;
pub use transcription::GroqTranscriptionProvider;
pub use vllm::VLLMProvider;
pub use zhipu::ZhipuProvider;

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

/// Get API key from environment variable or config
fn get_api_key_from_env(name: &str, config_key: &str, config: &Config) -> Option<String> {
    // Check environment variable first
    if let Ok(key) = std::env::var(name) {
        if !key.is_empty() {
            return Some(key);
        }
    }
    // Fall back to config
    let key = match config_key {
        "openrouter" => &config.providers.openrouter.api_key,
        "anthropic" => &config.providers.anthropic.api_key,
        "openai" => &config.providers.openai.api_key,
        "groq" => &config.providers.groq.api_key,
        "gemini" => &config.providers.gemini.api_key,
        "minimax" => &config.providers.minimax.api_key,
        "deepseek" => &config.providers.deepseek.api_key,
        "zhipu" => &config.providers.zhipu.api_key,
        "moonshot" => &config.providers.moonshot.api_key,
        "vllm" => &config.providers.vllm.api_key,
        _ => return None,
    };
    if !key.is_empty() {
        Some(key.clone())
    } else {
        None
    }
}

/// Create a provider based on configuration priority
pub fn create_provider(config: &Config) -> Box<dyn LLMProvider> {
    // Debug: print api key status
    tracing::debug!("openrouter api_key empty: {}", config.providers.openrouter.api_key.is_empty());
    tracing::debug!("anthropic api_key empty: {}", config.providers.anthropic.api_key.is_empty());
    tracing::debug!("openai api_key empty: {}", config.providers.openai.api_key.is_empty());
    tracing::debug!("groq api_key empty: {}", config.providers.groq.api_key.is_empty());
    tracing::debug!("gemini api_key empty: {}", config.providers.gemini.api_key.is_empty());
    tracing::debug!("minimax api_key empty: {}", config.providers.minimax.api_key.is_empty());
    tracing::debug!("minimax api_key: {}", if config.providers.minimax.api_key.len() > 10 { &config.providers.minimax.api_key[..10] } else { &config.providers.minimax.api_key });

    // Priority: OpenRouter > Anthropic > OpenAI > Groq > Gemini > MiniMax > DeepSeek > Zhipu > Moonshot
    if let Some(key) = get_api_key_from_env("OPENROUTER_API_KEY", "openrouter", config) {
        tracing::debug!("Using OpenRouter from env");
        return Box::new(OpenRouterProvider::new(key));
    }
    if let Some(key) = get_api_key_from_env("ANTHROPIC_API_KEY", "anthropic", config) {
        return Box::new(AnthropicProvider::new(key));
    }
    if let Some(key) = get_api_key_from_env("OPENAI_API_KEY", "openai", config) {
        return Box::new(OpenAIProvider::new(key, config.providers.openai.api_base.clone()));
    }
    if let Some(key) = get_api_key_from_env("GROQ_API_KEY", "groq", config) {
        return Box::new(GroqProvider::new(key));
    }
    if let Some(key) = get_api_key_from_env("GEMINI_API_KEY", "gemini", config) {
        return Box::new(GeminiProvider::new(key));
    }
    if let Some(key) = get_api_key_from_env("MINIMAX_API_KEY", "minimax", config) {
        return Box::new(MiniMaxProvider::new(key));
    }
    if let Some(key) = get_api_key_from_env("DEEPSEEK_API_KEY", "deepseek", config) {
        return Box::new(DeepSeekProvider::new(key, None));
    }
    if let Some(key) = get_api_key_from_env("ZHIPU_API_KEY", "zhipu", config) {
        return Box::new(zhipu::ZhipuProvider::new(key, None));
    }
    if let Some(key) = get_api_key_from_env("MOONSHOT_API_KEY", "moonshot", config) {
        return Box::new(moonshot::MoonshotProvider::new(key, None));
    }

    // Fall back to config file values
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
        Box::new(minimax::MiniMaxProvider::new(config.providers.minimax.api_key.clone()))
    } else if !config.providers.deepseek.api_key.is_empty() {
        Box::new(DeepSeekProvider::new(config.providers.deepseek.api_key.clone(), None))
    } else if !config.providers.zhipu.api_key.is_empty() {
        Box::new(zhipu::ZhipuProvider::new(config.providers.zhipu.api_key.clone(), None))
    } else if !config.providers.moonshot.api_key.is_empty() {
        Box::new(moonshot::MoonshotProvider::new(config.providers.moonshot.api_key.clone(), None))
    } else {
        // No provider configured - return a dummy provider that returns an error
        Box::new(OpenRouterProvider::new(String::new()))
    }
}
