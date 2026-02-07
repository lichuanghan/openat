//! MiniMax provider - Chinese LLM.

use crate::types::{LLMResponse, ToolCall};
use crate::llm::providers::LLMProvider;
use serde_json::{json, Value};

/// MiniMax provider
#[derive(Debug)]
pub struct MiniMaxProvider {
    api_key: String,
    api_base: String,
}

impl MiniMaxProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            api_base: "https://api.minimax.chat/v1/text/chatcompletion_v2".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for MiniMaxProvider {
    async fn chat(
        &self,
        messages: &[Value],
        model: &str,
        tools: &[Value],
    ) -> Result<LLMResponse, String> {
        let client = reqwest::Client::new();

        // MiniMax expects model ID without provider prefix
        let model_id = model.split('/').last().unwrap_or(model);

        // MiniMax uses OpenAI-compatible API
        let body = json!({
            "model": model_id,
            "messages": messages,
            "tools": tools,
            "tool_choice": if tools.is_empty() { json!(null) } else { json!("auto") }
        });

        let response = client
            .post(&self.api_base)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("X-Api-Key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let status = response.status();
        tracing::debug!("MiniMax response status: {}", status);

        let text = response.text().await.map_err(|e| format!("Read error: {}", e))?;
        tracing::debug!("MiniMax response body: {}", text);

        if !status.is_success() {
            return Err(format!("API error (status {}): {}", status, text));
        }

        // Parse the text as JSON
        let response_json: Value = serde_json::from_str(&text)
            .map_err(|e| format!("Parse error: {}", e))?;

        let choice = &response_json["choices"][0];
        let content = choice["message"]["content"].as_str().map(|s| s.to_string());

        let tool_calls: Vec<ToolCall> = if let Some(tc_array) = choice["message"]["tool_calls"].as_array() {
            tc_array.iter().map(|tc| ToolCall {
                id: tc["id"].as_str().unwrap_or("").to_string(),
                name: tc["function"]["name"].as_str().unwrap_or("").to_string(),
                arguments: tc["function"]["arguments"].clone(),
            }).collect()
        } else {
            vec![]
        };

        let finish_reason = choice["finish_reason"]
            .as_str()
            .unwrap_or("stop")
            .to_string();

        Ok(LLMResponse {
            content,
            tool_calls,
            finish_reason,
        })
    }

    fn name(&self) -> &str {
        "minimax"
    }

    fn api_base(&self) -> &str {
        &self.api_base
    }
}
