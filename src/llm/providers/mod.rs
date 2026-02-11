//! LLM providers implementations.

mod anthropic;
mod deepseek;
mod gemini;
mod groq;
mod minimax;
mod moonshot;
mod openai;
mod openai_compat;
mod openrouter;
mod transcription;
mod vllm;
mod zhipu;

pub use openai_compat::OpenAICompatConfig;

pub use anthropic::AnthropicProvider;
pub use deepseek::DeepSeekProvider;
pub use gemini::GeminiProvider;
pub use groq::GroqProvider;
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

/// Get API key from env or config - helper function
fn get_api_key(env_key: &str, config: &Config) -> Option<String> {
    std::env::var(env_key).ok().filter(|k| !k.is_empty())
}

/// Create a provider based on configuration priority
pub fn create_provider(config: &Config) -> Box<dyn LLMProvider> {
    // Priority: OpenRouter > Anthropic > OpenAI > Groq > Gemini > MiniMax > DeepSeek > Zhipu > Moonshot
    if let Some(key) = get_api_key("OPENROUTER_API_KEY", config) {
        tracing::debug!("Using OpenRouter from env");
        return Box::new(OpenRouterProvider::new(key));
    }
    if let Some(key) = get_api_key("ANTHROPIC_API_KEY", config) {
        return Box::new(AnthropicProvider::new(key));
    }
    if let Some(key) = get_api_key("OPENAI_API_KEY", config) {
        return Box::new(OpenAIProvider::new(key, config.providers.openai.api_base.clone()));
    }
    if let Some(key) = get_api_key("GROQ_API_KEY", config) {
        return Box::new(GroqProvider::new(key));
    }
    if let Some(key) = get_api_key("GEMINI_API_KEY", config) {
        return Box::new(GeminiProvider::new(key));
    }
    if let Some(key) = get_api_key("MINIMAX_API_KEY", config) {
        return Box::new(MiniMaxProvider::new(key));
    }
    if let Some(key) = get_api_key("DEEPSEEK_API_KEY", config) {
        return Box::new(DeepSeekProvider::new(key, None));
    }
    if let Some(key) = get_api_key("ZHIPU_API_KEY", config) {
        return Box::new(zhipu::ZhipuProvider::new(key, None));
    }
    if let Some(key) = get_api_key("MOONSHOT_API_KEY", config) {
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
        // No provider configured
        Box::new(OpenRouterProvider::new(String::new()))
    }
}
