//! Moonshot (月之暗面) provider - Kimi API.

use crate::llm::providers::openai_compat::OpenAICompatConfig;
use crate::types::LLMResponse;
use crate::llm::providers::LLMProvider;
use serde_json::Value;

/// Moonshot (月之暗面) provider
#[derive(Debug, Clone)]
pub struct MoonshotProvider {
    config: OpenAICompatConfig,
    default_model: String,
}

impl MoonshotProvider {
    pub fn new(api_key: String, api_base: Option<String>) -> Self {
        Self {
            config: OpenAICompatConfig::new(
                api_key,
                api_base.unwrap_or_else(|| "https://api.moonshot.cn/v1".to_string()),
                "moonshot",
            ).with_header("Content-Type", "application/json".to_string()),
            default_model: "moonshot-v1-8k".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for MoonshotProvider {
    async fn chat(
        &self,
        messages: &[Value],
        model: &str,
        tools: &[Value],
    ) -> Result<LLMResponse, String> {
        let model_name = if model.is_empty() || model.starts_with("moonshot-") || model.starts_with("kimi") {
            self.default_model.clone()
        } else {
            model.to_string()
        };
        self.config.chat_impl(messages, &model_name, tools).await
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
    fn test_moonshot_provider_default() {
        let provider = MoonshotProvider::new("test-key".to_string(), None);
        assert_eq!(provider.name(), "moonshot");
        assert_eq!(provider.api_base(), "https://api.moonshot.cn/v1");
    }

    #[test]
    fn test_moonshot_provider_custom_api_base() {
        let provider = MoonshotProvider::new(
            "test-key".to_string(),
            Some("https://custom.api.com".to_string())
        );
        assert_eq!(provider.api_base(), "https://custom.api.com");
    }
}
