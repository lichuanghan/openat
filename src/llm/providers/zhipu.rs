//! Zhipu (智谱) provider - ChatGLM API.

use crate::llm::providers::openai_compat::OpenAICompatConfig;
use crate::types::LLMResponse;
use crate::llm::providers::LLMProvider;
use serde_json::Value;

/// Zhipu (智谱) provider
#[derive(Debug, Clone)]
pub struct ZhipuProvider {
    config: OpenAICompatConfig,
    default_model: String,
}

impl ZhipuProvider {
    pub fn new(api_key: String, api_base: Option<String>) -> Self {
        Self {
            config: OpenAICompatConfig::new(
                api_key,
                api_base.unwrap_or_else(|| "https://open.bigmodel.cn/api/paas/v4".to_string()),
                "zhipu",
            ).with_header("Content-Type", "application/json".to_string()),
            default_model: "glm-4".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for ZhipuProvider {
    async fn chat(
        &self,
        messages: &[Value],
        model: &str,
        tools: &[Value],
    ) -> Result<LLMResponse, String> {
        let model_name = if model.is_empty() || model.starts_with("glm-") || model.starts_with("chatglm") {
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
    fn test_zhipu_provider_default() {
        let provider = ZhipuProvider::new("test-key".to_string(), None);
        assert_eq!(provider.name(), "zhipu");
        assert!(provider.api_base().contains("bigmodel.cn"));
    }

    #[test]
    fn test_zhipu_provider_custom_api_base() {
        let provider = ZhipuProvider::new(
            "test-key".to_string(),
            Some("https://custom.api.com".to_string())
        );
        assert_eq!(provider.api_base(), "https://custom.api.com");
    }
}
