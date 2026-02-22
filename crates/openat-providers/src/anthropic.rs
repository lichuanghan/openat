//! Anthropic provider (Claude)

use serde_json::json;

use crate::{LLMProvider, LLMResponse};
use super::openai_compat::OpenAICompatConfig;

/// Anthropic provider
#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    config: OpenAICompatConfig,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Self {
        let api_key_clone = api_key.clone();
        Self {
            config: OpenAICompatConfig::new(
                api_key,
                "https://api.anthropic.com/v1".to_string(),
                "anthropic",
            ).with_header("x-api-key", api_key_clone)
             .with_header("anthropic-version", "2023-06-01".to_string()),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for AnthropicProvider {
    async fn chat(
        &self,
        messages: &[serde_json::Value],
        model: &str,
        _tools: &[serde_json::Value],
    ) -> Result<LLMResponse, String> {
        // Anthropic uses "Claude" prefix for models
        let model_name = if model.is_empty() || model.starts_with("claude") {
            "claude-sonnet-4-20250514".to_string()
        } else {
            model.to_string()
        };

        let client = reqwest::Client::new();
        let body = json!({
            "model": model_name,
            "messages": messages,
            "max_tokens": 1024,
        });

        let request = client
            .post("https://api.anthropic.com/v1/messages")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body);

        let response = request
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("Anthropic API error: {}", error));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;

        let content = response_json["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string());

        Ok(LLMResponse {
            content,
            tool_calls: vec![],
            finish_reason: "stop".to_string(),
        })
    }

    fn name(&self) -> &str {
        self.config.name
    }

    fn api_base(&self) -> &str {
        &self.config.api_base
    }
}
