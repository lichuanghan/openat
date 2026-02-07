//! LiteLLM provider - Unified multi-provider support.
//!
//! Supports OpenAI, Anthropic, Gemini, Groq, DeepSeek, Moonshot, Zhipu, and custom endpoints
//! through a unified OpenAI-compatible interface.

use crate::types::{LLMResponse, ToolCall};
use crate::llm::providers::LLMProvider;
use serde_json::{json, Value};

/// LiteLLM Provider
#[derive(Debug, Clone)]
pub struct LiteLLMProvider {
    api_key: String,
    api_base: String,
    default_model: String,
}

impl LiteLLMProvider {
    pub fn new(api_key: Option<String>, api_base: Option<String>, default_model: String) -> Self {
        Self {
            api_key: api_key.unwrap_or_default(),
            api_base: api_base.unwrap_or_default(),
            default_model,
        }
    }

    /// Detect provider type from model name and api_base
    fn detect_provider_type(&self) -> &str {
        let model = self.default_model.to_lowercase();
        let api_base = self.api_base.to_lowercase();

        if api_base.contains("openrouter") {
            return "openrouter";
        }
        if api_base.contains("vllm") || api_base.contains("tgi") {
            return "vllm";
        }
        if model.starts_with("anthropic/") || model.contains("claude") {
            return "anthropic";
        }
        if model.starts_with("gemini/") {
            return "gemini";
        }
        if model.starts_with("groq/") {
            return "groq";
        }
        if model.starts_with("deepseek/") || model.contains("deepseek") {
            return "deepseek";
        }
        if model.starts_with("moonshot/") || model.contains("kimi") {
            return "moonshot";
        }
        if model.starts_with("zhipu/") || model.starts_with("zai/") || model.starts_with("glm-") {
            return "zhipu";
        }
        if model.starts_with("openai/") || model.starts_with("gpt-") {
            return "openai";
        }

        "openai"
    }

    /// Get API base URL for provider
    fn get_api_base(&self) -> String {
        if !self.api_base.is_empty() {
            return self.api_base.clone();
        }

        match self.detect_provider_type() {
            "openrouter" => "https://openrouter.ai/api/v1".to_string(),
            "anthropic" => "https://api.anthropic.com/v1".to_string(),
            "gemini" => "https://api.google.com/v1".to_string(),
            "groq" => "https://api.groq.com/openai/v1".to_string(),
            "deepseek" => "https://api.deepseek.com/chat".to_string(),
            "moonshot" => "https://api.moonshot.cn/v1".to_string(),
            "zhipu" => "https://open.bigmodel.cn/api/paas/v4".to_string(),
            "vllm" => "http://localhost:8000/v1".to_string(),
            "openai" | _ => "https://api.openai.com/v1".to_string(),
        }
    }

    /// Normalize model name for provider
    fn normalize_model(&self, model: &str) -> String {
        let provider = self.detect_provider_type();
        let model = model.trim().to_string();

        match provider {
            "openrouter" => {
                if !model.starts_with("openrouter/") && !model.starts_with("anthropic/") {
                    format!("openrouter/{}", model)
                } else {
                    model
                }
            }
            "anthropic" => model.strip_prefix("anthropic/").unwrap_or(&model).to_string(),
            "gemini" => {
                if model.to_lowercase().starts_with("gemini/") {
                    model
                } else {
                    format!("gemini/{}", model)
                }
            }
            "moonshot" => {
                if model.to_lowercase().starts_with("moonshot/") {
                    model
                } else {
                    format!("moonshot/{}", model)
                }
            }
            "zhipu" => {
                if model.starts_with("zhipu/") || model.starts_with("zai/") {
                    model
                } else {
                    format!("zai/{}", model)
                }
            }
            "vllm" => {
                if model.starts_with("hosted_vllm/") {
                    model
                } else {
                    format!("hosted_vllm/{}", model)
                }
            }
            _ => model,
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for LiteLLMProvider {
    async fn chat(
        &self,
        messages: &[Value],
        model: &str,
        tools: &[Value],
    ) -> Result<LLMResponse, String> {
        let client = reqwest::Client::new();

        let model = self.normalize_model(model);
        let api_base = self.get_api_base();
        let provider = self.detect_provider_type();

        let mut body = json!({
            "model": model,
            "messages": messages,
        });

        if !tools.is_empty() {
            body["tools"] = json!(tools);
            body["tool_choice"] = json!("auto");
        }

        // Provider-specific adjustments
        if provider == "moonshot" && model.to_lowercase().contains("kimi-k2.5") {
            body["temperature"] = json!(1.0);
        }

        let url = format!("{}/chat/completions", api_base);

        let mut request = client.post(&url);

        // Set authorization header based on provider
        request = request.header("Authorization", format!("Bearer {}", self.api_key));

        match provider {
            "openrouter" => {
                if self.api_key.starts_with("sk-or-") {
                    request = request.header("HTTP-Referer", "https://github.com/openai/openai-python");
                }
            }
            "anthropic" => {
                request = request.header("x-api-key", &self.api_key);
                request = request.header("Anthropic-Version", "2023-06-01");
            }
            _ => {}
        };

        let response = request
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
        "litellm"
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

    let choice = if let Some(choices) = response_json.get("choices") {
        &choices[0]
    } else if let Some(candidates) = response_json.get("candidates") {
        &candidates[0]["content"]
    } else {
        return Err("Invalid response format".to_string());
    };

    let content = if let Some(text) = choice["message"]["text"].as_str() {
        Some(text.to_string())
    } else if let Some(content) = choice["message"]["content"].as_str() {
        Some(content.to_string())
    } else {
        choice["message"]["content"].as_str().map(|s| s.to_string())
    };

    let tool_calls: Vec<ToolCall> = if let Some(tc_array) = choice["message"]["tool_calls"].as_array() {
        tc_array.iter().map(|tc| ToolCall {
            id: tc["id"].as_str().unwrap_or("").to_string(),
            name: tc["function"]["name"].as_str().unwrap_or("").to_string(),
            arguments: tc["function"]["arguments"].clone(),
        }).collect()
    } else {
        vec![]
    };

    let finish_reason = if let Some(reason) = choice["finish_reason"].as_str() {
        reason.to_string()
    } else if let Some(stop_reason) = choice["stop_reason"].as_str() {
        stop_reason.to_string()
    } else {
        "stop".to_string()
    };

    Ok(LLMResponse {
        content,
        tool_calls,
        finish_reason,
    })
}
