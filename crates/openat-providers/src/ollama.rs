//! Ollama provider for local LLM inference

use reqwest::Client;
use serde_json::{json, Value};

use crate::{LLMProvider, LLMResponse};

/// Ollama provider - OpenAI-compatible API for local models
#[derive(Debug, Clone)]
pub struct OllamaProvider {
    api_base: String,
    api_key: String,
    default_model: String,
    client: Client,
}

impl OllamaProvider {
    /// Create a new Ollama provider
    pub fn new(api_base: Option<String>, api_key: Option<String>, default_model: Option<String>) -> Self {
        Self {
            api_base: api_base.unwrap_or_else(|| "http://localhost:11434".to_string()),
            api_key: api_key.unwrap_or_else(|| "ollama".to_string()),
            default_model: default_model.unwrap_or_else(|| "llama2".to_string()),
            client: Client::new(),
        }
    }

    fn chat_url(&self) -> String {
        format!("{}/v1/chat/completions", self.api_base)
    }

    async fn chat_impl(
        &self,
        messages: &[Value],
        model: &str,
        tools: &[Value],
    ) -> Result<LLMResponse, String> {
        // Use model or default
        let model_name = if model.is_empty() || model.starts_with("ollama/") {
            // Extract model name from "ollama/xxx" or use default
            if model.starts_with("ollama/") {
                model.strip_prefix("ollama/").unwrap_or(&self.default_model)
            } else {
                &self.default_model
            }
        } else {
            model
        };

        let body = json!({
            "model": model_name,
            "messages": messages,
            "tools": tools,
            "tool_choice": if tools.is_empty() { json!(null) } else { json!("auto") }
        });

        let response = self.client
            .post(&self.chat_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("Ollama API error: {}", error));
        }

        Self::parse_response(response).await
    }

    async fn parse_response(response: reqwest::Response) -> Result<LLMResponse, String> {
        let response_json: Value = response
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;

        let choice = &response_json["choices"][0];
        let content = choice["message"]["content"].as_str().map(|s| s.to_string());

        let tool_calls: Vec<crate::openai_compat::ToolCall> =
            if let Some(tc_array) = choice["message"]["tool_calls"].as_array() {
                tc_array
                    .iter()
                    .map(|tc| crate::openai_compat::ToolCall {
                        id: tc["id"].as_str().unwrap_or("").to_string(),
                        name: tc["function"]["name"].as_str().unwrap_or("").to_string(),
                        arguments: tc["function"]["arguments"].clone(),
                    })
                    .collect()
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
}

#[async_trait::async_trait]
impl LLMProvider for OllamaProvider {
    async fn chat(
        &self,
        messages: &[Value],
        model: &str,
        tools: &[Value],
    ) -> Result<LLMResponse, String> {
        self.chat_impl(messages, model, tools).await
    }

    fn name(&self) -> &str {
        "ollama"
    }

    fn api_base(&self) -> &str {
        &self.api_base
    }
}
