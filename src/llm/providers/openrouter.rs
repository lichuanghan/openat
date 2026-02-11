//! OpenRouter provider - OpenAI-compatible API.

use crate::llm::providers::openai_compat::OpenAICompatConfig;
use crate::types::LLMResponse;
use crate::llm::providers::LLMProvider;
use serde_json::Value;

/// OpenRouter provider
#[derive(Debug, Clone)]
pub struct OpenRouterProvider {
    config: OpenAICompatConfig,
}

impl OpenRouterProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            config: OpenAICompatConfig::new(
                api_key,
                "https://openrouter.ai/api/v1".to_string(),
                "openrouter",
            ).with_header("HTTP-Referer", "https://github.com/HKUDS/openat".to_string())
             .with_header("X-Title", "openat".to_string()),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for OpenRouterProvider {
    async fn chat(
        &self,
        messages: &[Value],
        model: &str,
        tools: &[Value],
    ) -> Result<LLMResponse, String> {
        self.config.chat_impl(messages, model, tools).await
    }

    fn name(&self) -> &str {
        self.config.name
    }

    fn api_base(&self) -> &str {
        &self.config.api_base
    }
}
