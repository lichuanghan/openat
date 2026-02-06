//! Anthropic provider - Claude API.

use crate::types::LLMResponse;
use crate::llm::providers::LLMProvider;
use serde_json::{json, Value};

/// Anthropic provider
#[derive(Debug)]
pub struct AnthropicProvider {
    api_key: String,
    api_base: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            api_base: "https://api.anthropic.com/v1".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for AnthropicProvider {
    async fn chat(
        &self,
        messages: &[Value],
        model: &str,
        tools: &[Value],
    ) -> Result<LLMResponse, String> {
        let client = reqwest::Client::new();

        // Convert messages to Anthropic format
        let anthropic_messages: Vec<Value> = messages.iter()
            .filter(|m| m["role"] != "system")
            .cloned()
            .collect();

        let system_message = messages.iter()
            .find(|m| m["role"] == "system")
            .and_then(|m| m["content"].as_str())
            .unwrap_or("");

        let body = json!({
            "model": model,
            "messages": anthropic_messages,
            "system": system_message,
            "tools": if tools.is_empty() { json!(null) } else { json!(tools) },
            "max_tokens": 4096
        });

        let response = client
            .post(&self.api_base)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("API error: {}", error));
        }

        parse_response(response).await
    }

    fn name(&self) -> &str {
        "anthropic"
    }

    fn api_base(&self) -> &str {
        &self.api_base
    }
}

async fn parse_response(response: reqwest::Response) -> Result<LLMResponse, String> {
    let response_json: Value = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let content = response_json["content"][0]["text"].as_str()
        .map(|s| s.to_string());

    let tool_calls: Vec<crate::types::ToolCall> = if let Some(tc_array) = response_json["content"].as_array() {
        tc_array.iter()
            .filter(|tc| tc["type"] == "tool_use")
            .map(|tc| crate::types::ToolCall {
                id: tc["id"].as_str().unwrap_or("").to_string(),
                name: tc["name"].as_str().unwrap_or("").to_string(),
                arguments: if tc["input"].is_null() || !tc["input"].is_object() {
                    json!({})
                } else {
                    tc["input"].clone()
                },
            })
            .collect()
    } else {
        vec![]
    };

    let finish_reason = response_json["stop_reason"]
        .as_str()
        .unwrap_or("stop")
        .to_string();

    Ok(LLMResponse {
        content,
        tool_calls,
        finish_reason,
    })
}
