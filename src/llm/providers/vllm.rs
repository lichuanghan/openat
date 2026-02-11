//! VLLM provider - OpenAI-compatible local LLM serving.

use crate::llm::providers::openai_compat::OpenAICompatConfig;
use crate::types::LLMResponse;
use crate::llm::providers::LLMProvider;
use serde_json::Value;

/// VLLM provider - for local LLM serving with OpenAI-compatible API
#[derive(Debug, Clone)]
pub struct VLLMProvider {
    config: OpenAICompatConfig,
    default_model: String,
}

impl VLLMProvider {
    pub fn new(api_key: String, api_base: Option<String>, default_model: Option<String>) -> Self {
        Self {
            config: OpenAICompatConfig::new(
                api_key,
                api_base.unwrap_or_else(|| "http://localhost:8000/v1".to_string()),
                "vllm",
            ),
            default_model: default_model.unwrap_or_else(|| "meta-llama/Llama-2-7b-hf".to_string()),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for VLLMProvider {
    async fn chat(
        &self,
        messages: &[Value],
        model: &str,
        tools: &[Value],
    ) -> Result<LLMResponse, String> {
        let model_name = if model.is_empty() {
            &self.default_model
        } else {
            model
        };
        self.config.chat_impl(messages, model_name, tools).await
    }

    fn name(&self) -> &str {
        self.config.name
    }

    fn api_base(&self) -> &str {
        &self.config.api_base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vllm_provider_default() {
        let provider = VLLMProvider::new("".to_string(), None, None);
        assert_eq!(provider.name(), "vllm");
        assert_eq!(provider.api_base(), "http://localhost:8000/v1");
        assert!(provider.default_model.contains("llama"));
    }

    #[test]
    fn test_vllm_provider_custom() {
        let provider = VLLMProvider::new(
            "test-key".to_string(),
            Some("https://vllm.example.com/v1".to_string()),
            Some("custom-model".to_string())
        );
        assert_eq!(provider.api_base(), "https://vllm.example.com/v1");
        assert_eq!(provider.default_model, "custom-model");
    }
}
