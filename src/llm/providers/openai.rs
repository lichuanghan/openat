//! OpenAI provider - GPT-4, GPT-3.5.

use crate::llm::providers::openai_compat::OpenAICompatConfig;
use crate::types::LLMResponse;
use crate::llm::providers::LLMProvider;
use serde_json::Value;

/// OpenAI provider
#[derive(Debug, Clone)]
pub struct OpenAIProvider {
    config: OpenAICompatConfig,
}

impl OpenAIProvider {
    pub fn new(api_key: String, api_base: Option<String>) -> Self {
        Self {
            config: OpenAICompatConfig::new(
                api_key,
                api_base.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
                "openai",
            ),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for OpenAIProvider {
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
