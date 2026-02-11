//! DeepSeek provider - DeepSeek Chat API.

use crate::llm::providers::openai_compat::OpenAICompatConfig;
use crate::types::LLMResponse;
use crate::llm::providers::LLMProvider;
use serde_json::{json, Value};

/// DeepSeek provider
#[derive(Debug, Clone)]
pub struct DeepSeekProvider {
    config: OpenAICompatConfig,
}

impl DeepSeekProvider {
    pub fn new(api_key: String, api_base: Option<String>) -> Self {
        Self {
            config: OpenAICompatConfig::new(
                api_key,
                api_base.unwrap_or_else(|| "https://api.deepseek.com".to_string()),
                "deepseek",
            )
            .with_header("Content-Type", "application/json".to_string()),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for DeepSeekProvider {
    async fn chat(
        &self,
        messages: &[Value],
        model: &str,
        tools: &[Value],
    ) -> Result<LLMResponse, String> {
        // DeepSeek uses "deepseek-chat" as default model
        let model_name = if model.is_empty() || model.starts_with("deepseek-") {
            "deepseek-chat".to_string()
        } else {
            model.to_string()
        };

        // Use the base implementation with our model
        let client = reqwest::Client::new();
        let config = &self.config;

        let body = json!({
            "model": model_name,
            "messages": messages,
            "tools": tools,
            "tool_choice": if tools.is_empty() { json!(null) } else { json!("auto") }
        });

        let mut request = client
            .post(&config.chat_url())
            .header("Authorization", config.auth_value());

        // Add extra headers
        for (key, value) in &config.extra_headers {
            request = request.header(*key, value);
        }

        let response = request
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("DeepSeek API error: {}", error));
        }

        crate::llm::providers::openai_compat::parse_openai_response(response).await
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
    fn test_deepseek_provider_default() {
        let provider = DeepSeekProvider::new("test-key".to_string(), None);
        assert_eq!(provider.name(), "deepseek");
        assert_eq!(provider.api_base(), "https://api.deepseek.com");
    }

    #[test]
    fn test_deepseek_provider_custom_api_base() {
        let provider = DeepSeekProvider::new(
            "test-key".to_string(),
            Some("https://custom.api.com".to_string())
        );
        assert_eq!(provider.api_base(), "https://custom.api.com");
    }
}
