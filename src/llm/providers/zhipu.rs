//! Zhipu (智谱) provider - ChatGLM API.

use crate::types::LLMResponse;
use crate::llm::providers::LLMProvider;
use serde_json::{json, Value};

/// Zhipu (智谱) provider
#[derive(Debug)]
pub struct ZhipuProvider {
    api_key: String,
    api_base: String,
}

impl ZhipuProvider {
    pub fn new(api_key: String, api_base: Option<String>) -> Self {
        // Zhipu's default API base
        let default_base = "https://open.bigmodel.cn/api/paas/v4".to_string();
        Self {
            api_key,
            api_base: api_base.unwrap_or(default_base),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for ZhipuProvider {
    async fn chat(
        &self,
        messages: &[Value],
        model: &str,
        tools: &[Value],
    ) -> Result<LLMResponse, String> {
        let client = reqwest::Client::new();

        // Zhipu uses glm-4 as default model
        let model_name = if model.is_empty()
            || model.starts_with("glm-")
            || model.starts_with("chatglm") {
            "glm-4".to_string()
        } else {
            model.to_string()
        };

        let body = json!({
            "model": model_name,
            "messages": messages,
            "tools": tools,
            "tool_choice": if tools.is_empty() { json!(null) } else { json!("auto") }
        });

        let response = client
            .post(&format!("{}/chat/completions", self.api_base))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("Zhipu API error: {}", error));
        }

        parse_response(response).await
    }

    fn name(&self) -> &str {
        "zhipu"
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
    fn test_zhipu_provider_default() {
        let provider = ZhipuProvider::new("test-key".to_string(), None);
        assert_eq!(provider.name(), "zhipu");
        assert!(provider.api_base.contains("bigmodel.cn"));
    }

    #[test]
    fn test_zhipu_provider_custom_api_base() {
        let provider = ZhipuProvider::new(
            "test-key".to_string(),
            Some("https://custom.api.com".to_string())
        );
        assert_eq!(provider.api_base, "https://custom.api.com");
    }
}
