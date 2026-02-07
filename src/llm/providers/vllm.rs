//! VLLM provider - OpenAI-compatible local LLM serving.

use crate::types::LLMResponse;
use crate::llm::providers::LLMProvider;
use serde_json::{json, Value};

/// VLLM provider - for local LLM serving with OpenAI-compatible API
#[derive(Debug)]
pub struct VLLMProvider {
    api_key: String,
    api_base: String,
    default_model: String,
}

impl VLLMProvider {
    pub fn new(api_key: String, api_base: Option<String>, default_model: Option<String>) -> Self {
        Self {
            api_key,
            api_base: api_base.unwrap_or_else(|| "http://localhost:8000/v1".to_string()),
            default_model: default_model.unwrap_or_else(|| "meta-llama/Llama-2-7b-hf".to_string()),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for VLLMProvider {
    async fn chat(
        &self,
        messages: &[Value],
        model: &str,
        tools: &[Value],
    ) -> Result<LLMResponse, String> {
        let client = reqwest::Client::new();

        // Use default model if none specified
        let model_name = if model.is_empty() {
            &self.default_model
        } else {
            model
        };

        let body = json!({
            "model": model_name,
            "messages": messages,
            "tools": tools,
            "tool_choice": if tools.is_empty() { json!(null) } else { json!("auto") }
        });

        // VLLM may not require authentication
        let mut request = client
            .post(&format!("{}/chat/completions", self.api_base))
            .header("Content-Type", "application/json")
            .json(&body);

        if !self.api_key.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", self.api_key));
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("VLLM API error: {}", error));
        }

        parse_response(response).await
    }

    fn name(&self) -> &str {
        "vllm"
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

    let choice = &response_json["choices"][0];
    let content = choice["message"]["content"].as_str().map(|s| s.to_string());

    let tool_calls: Vec<crate::types::ToolCall> = if let Some(tc_array) = choice["message"]["tool_calls"].as_array() {
        tc_array.iter().map(|tc| crate::types::ToolCall {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vllm_provider_default() {
        let provider = VLLMProvider::new("".to_string(), None, None);
        assert_eq!(provider.name(), "vllm");
        assert_eq!(provider.api_base, "http://localhost:8000/v1");
        assert!(provider.default_model.contains("llama"));
    }

    #[test]
    fn test_vllm_provider_custom() {
        let provider = VLLMProvider::new(
            "test-key".to_string(),
            Some("https://vllm.example.com/v1".to_string()),
            Some("custom-model".to_string())
        );
        assert_eq!(provider.api_base, "https://vllm.example.com/v1");
        assert_eq!(provider.default_model, "custom-model");
    }
}
