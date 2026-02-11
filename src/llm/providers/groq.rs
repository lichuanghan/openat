//! Groq provider - Fast inference.

use crate::llm::providers::openai_compat::OpenAICompatConfig;
use crate::types::LLMResponse;
use crate::llm::providers::LLMProvider;
use serde_json::Value;

/// Groq provider
#[derive(Debug, Clone)]
pub struct GroqProvider {
    config: OpenAICompatConfig,
}

impl GroqProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            config: OpenAICompatConfig::new(
                api_key,
                "https://api.groq.com/openai/v1".to_string(),
                "groq",
            ),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for GroqProvider {
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
