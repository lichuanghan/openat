//! OpenAI-compatible provider utilities.
//!
//! This module provides shared functionality for providers that use the OpenAI
//! chat completions API format (OpenAI, Groq, DeepSeek, Zhipu, Moonshot, etc.)

use crate::types::{LLMResponse, ToolCall};
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Shared response parser for OpenAI-compatible APIs
pub async fn parse_openai_response(response: reqwest::Response) -> Result<LLMResponse, String> {
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

/// Base configuration for OpenAI-compatible providers
#[derive(Debug, Clone)]
pub struct OpenAICompatConfig {
    pub api_key: String,
    pub api_base: String,
    pub name: &'static str,
    pub extra_headers: HashMap<&'static str, String>,
}

impl OpenAICompatConfig {
    /// Create a new config
    pub fn new(api_key: String, api_base: String, name: &'static str) -> Self {
        Self {
            api_key,
            api_base,
            name,
            extra_headers: HashMap::new(),
        }
    }

    /// Add extra header
    pub fn with_header(mut self, key: &'static str, value: String) -> Self {
        self.extra_headers.insert(key, value);
        self
    }

    /// Get chat completions URL
    pub fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.api_base)
    }

    /// Build auth header value
    pub fn auth_value(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    /// Chat implementation for LLMProvider trait
    pub async fn chat_impl(
        &self,
        messages: &[Value],
        model: &str,
        tools: &[Value],
    ) -> Result<LLMResponse, String> {
        chat_request(self, messages, model, tools).await
    }
}

/// Helper function to perform chat request (used by providers)
pub async fn chat_request(
    config: &OpenAICompatConfig,
    messages: &[Value],
    model: &str,
    tools: &[Value],
) -> Result<LLMResponse, String> {
    let client = Client::new();

    let body = json!({
        "model": model,
        "messages": messages,
        "tools": tools,
        "tool_choice": if tools.is_empty() { json!(null) } else { json!("auto") }
    });

    let mut request = client
        .post(&config.chat_url())
        .header("Authorization", config.auth_value())
        .json(&body);

    // Add extra headers
    for (key, value) in &config.extra_headers {
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

    parse_openai_response(response).await
}

/// Helper to build messages array from internal format
pub fn build_messages(messages: &[Value], system_prompt: Option<&str>) -> Vec<Value> {
    let mut result = Vec::new();

    if let Some(sys) = system_prompt {
        result.push(json!({
            "role": "system",
            "content": sys
        }));
    }

    for msg in messages {
        result.push(msg.clone());
    }

    result
}

/// Extract tool call arguments from JSON string or object
pub fn extract_tool_args(args: &Value) -> Value {
    if args.is_string() {
        let arg_str = args.as_str().unwrap_or("");
        if arg_str.starts_with('{') {
            serde_json::from_str(arg_str).unwrap_or_else(|_| json!({}))
        } else {
            args.clone()
        }
    } else {
        args.clone()
    }
}

/// Macro to create a simple OpenAI-compatible provider
#[macro_export]
macro_rules! make_openai_provider {
    ($name:ident, $provider_name:expr, $default_base:expr) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            config: $crate::llm::providers::openai_compat::OpenAICompatConfig,
        }

        impl $name {
            pub fn new(api_key: String, api_base: Option<String>) -> Self {
                Self {
                    config: $crate::llm::providers::openai_compat::OpenAICompatConfig::new(
                        api_key,
                        api_base.unwrap_or_else(|| $default_base.to_string()),
                        $provider_name,
                    ),
                }
            }
        }

        #[async_trait::async_trait]
        impl $crate::llm::providers::LLMProvider for $name {
            async fn chat(
                &self,
                messages: &[serde_json::Value],
                model: &str,
                tools: &[serde_json::Value],
            ) -> Result<$crate::types::LLMResponse, String> {
                $crate::llm::providers::openai_compat::chat_request(
                    &self.config,
                    messages,
                    model,
                    tools,
                )
                .await
            }

            fn name(&self) -> &str {
                self.config.name
            }

            fn api_base(&self) -> &str {
                &self.config.api_base
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_new() {
        let config = OpenAICompatConfig::new(
            "test-key".to_string(),
            "https://api.example.com/v1".to_string(),
            "test",
        );
        assert_eq!(config.name, "test");
        assert_eq!(config.api_key, "test-key");
        assert!(config.extra_headers.is_empty());
    }

    #[test]
    fn test_config_with_header() {
        let config = OpenAICompatConfig::new(
            "test-key".to_string(),
            "https://api.example.com/v1".to_string(),
            "test",
        )
        .with_header("X-Custom", "value".to_string());

        assert_eq!(config.extra_headers.get("X-Custom"), Some(&"value".to_string()));
    }

    #[test]
    fn test_chat_url() {
        let config = OpenAICompatConfig::new(
            "test-key".to_string(),
            "https://api.example.com/v1".to_string(),
            "test",
        );
        assert_eq!(config.chat_url(), "https://api.example.com/v1/chat/completions");
    }

    #[test]
    fn test_auth_value() {
        let config = OpenAICompatConfig::new(
            "my-key".to_string(),
            "https://api.example.com/v1".to_string(),
            "test",
        );
        assert_eq!(config.auth_value(), "Bearer my-key");
    }
}
