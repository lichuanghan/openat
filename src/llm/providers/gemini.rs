//! Gemini provider - Google AI.

use crate::types::LLMResponse;
use crate::llm::providers::LLMProvider;
use serde_json::{json, Value};

/// Gemini provider
#[derive(Debug)]
pub struct GeminiProvider {
    api_key: String,
    api_base: String,
}

impl GeminiProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            api_base: "https://generativelanguage.googleapis.com/v1beta".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for GeminiProvider {
    async fn chat(
        &self,
        messages: &[Value],
        model: &str,
        _tools: &[Value],
    ) -> Result<LLMResponse, String> {
        let client = reqwest::Client::new();

        // Gemini has different format
        let contents: Vec<Value> = messages.iter()
            .filter(|m| m["role"] != "system")
            .map(|m| {
                json!({
                    "role": if m["role"] == "user" { "user" } else { "model" },
                    "parts": [{
                        "text": m["content"]
                    }]
                })
            })
            .collect();

        let system_instruction = messages.iter()
            .find(|m| m["role"] == "system")
            .map(|m| json!({ "text": m["content"] }))
            .unwrap_or(json!({}));

        // Extract model name from format "gemini/gemini-pro"
        let model_name = model.split('/').last().unwrap_or(model);

        let url = format!("{}/models/{}:generateContent?key={}",
            self.api_base, model_name, self.api_key);

        let body = json!({
            "contents": contents,
            "system_instruction": system_instruction
        });

        let response = client
            .post(&url)
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
        "gemini"
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

    let content = response_json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_default();

    // Gemini doesn't support tool calls in the same way
    Ok(LLMResponse {
        content: Some(content),
        tool_calls: vec![],
        finish_reason: "stop".to_string(),
    })
}
