//! OpenAI-compatible provider utilities

use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::{LLMProvider, LLMResponse};

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
