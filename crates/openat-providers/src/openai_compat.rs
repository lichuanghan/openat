//! OpenAI-compatible provider utilities

use futures_util::Stream;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::pin::Pin;

use crate::{LLMProvider, LLMResponse, StreamResponse};

/// Tool call from LLM
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

/// OpenAI-compatible config
#[derive(Debug, Clone)]
pub struct OpenAICompatConfig {
    pub api_key: String,
    pub api_base: String,
    pub name: &'static str,
    pub extra_headers: HashMap<&'static str, String>,
}

impl OpenAICompatConfig {
    pub fn new(api_key: String, api_base: String, name: &'static str) -> Self {
        Self {
            api_key,
            api_base,
            name,
            extra_headers: HashMap::new(),
        }
    }

    pub fn with_header(mut self, key: &'static str, value: String) -> Self {
        self.extra_headers.insert(key, value);
        self
    }

    pub fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.api_base)
    }

    pub fn auth_value(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    pub async fn chat_impl(
        &self,
        messages: &[Value],
        model: &str,
        tools: &[Value],
    ) -> Result<LLMResponse, String> {
        let client = Client::new();

        // Extract model name (remove provider prefix like "minimax/")
        let model_name = model.split('/').last().unwrap_or(model);

        let body = json!({
            "model": model_name,
            "messages": messages,
            "tools": tools,
            "tool_choice": if tools.is_empty() { json!(null) } else { json!("auto") }
        });

        let mut request = client
            .post(&self.chat_url())
            .header("Authorization", self.auth_value())
            .json(&body);

        for (key, value) in &self.extra_headers {
            request = request.header(*key, value);
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("API error: {}", error));
        }

        Self::parse_response(response).await
    }

    /// Streaming chat implementation
    /// Note: Simplified implementation - makes non-streaming request and yields as chunks
    pub fn stream_impl(
        &self,
        messages: &[Value],
        model: &str,
        tools: &[Value],
    ) -> Pin<Box<dyn Stream<Item = Result<StreamResponse, String>> + Send>> {
        let model_name = model.split('/').last().unwrap_or(model).to_string();
        let api_base = self.api_base.clone();
        let auth = self.auth_value();
        let extra_headers = self.extra_headers.clone();

        let body = json!({
            "model": model_name,
            "messages": messages,
            "tools": tools,
            "tool_choice": if tools.is_empty() { json!(null) } else { json!("auto") }
        });

        Box::pin(async_stream::try_stream! {
            let client = Client::new();
            let mut request = client
                .post(format!("{}/chat/completions", api_base))
                .header("Authorization", auth)
                .header("Content-Type", "application/json")
                .json(&body);

            for (key, value) in &extra_headers {
                request = request.header(*key, value);
            }

            let response = request
                .send()
                .await
                .map_err(|e| format!("Stream request failed: {}", e))?;

            let status = response.status();
            let body_bytes = response.bytes().await
                .map_err(|e| format!("Read response: {}", e))?;

            if !status.is_success() {
                let error = String::from_utf8_lossy(&body_bytes);
                Err(format!("Stream API error {}: {}", status, error))?;
            }

            let response_json: Value = serde_json::from_slice(&body_bytes)
                .map_err(|e| format!("Parse error: {}", e))?;

            let choice = &response_json["choices"][0];
            let content = choice["message"]["content"].as_str().unwrap_or("");

            // Yield content in chunks for typing effect
            let chars: Vec<char> = content.chars().collect();
            let chunk_size = 2; // Smaller chunks for typing effect
            let mut i = 0;
            while i < chars.len() {
                let end = std::cmp::min(i + chunk_size, chars.len());
                let chunk: String = chars[i..end].iter().collect();
                i = end;

                yield StreamResponse {
                    content: chunk,
                    is_final: false,
                    tool_calls: vec![],
                };

                // Typing delay - natural reading speed
                tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            }

            yield StreamResponse {
                content: String::new(),
                is_final: true,
                tool_calls: vec![],
            };
        })
    }

    async fn parse_response(response: reqwest::Response) -> Result<LLMResponse, String> {
        let response_json: Value = response
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;

        let choice = &response_json["choices"][0];
        let content = choice["message"]["content"].as_str().map(|s| s.to_string());

        let tool_calls: Vec<ToolCall> = if let Some(tc_array) = choice["message"]["tool_calls"].as_array() {
            tc_array
                .iter()
                .map(|tc| ToolCall {
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

/// OpenAI-compatible provider
#[derive(Debug, Clone)]
pub struct OpenAICompatProvider {
    config: OpenAICompatConfig,
}

impl OpenAICompatProvider {
    pub fn new(config: OpenAICompatConfig) -> Self {
        Self { config }
    }
}

#[async_trait::async_trait]
impl LLMProvider for OpenAICompatProvider {
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
