use crate::config::Config;
use serde_json::{json, Value};

/// Trait for LLM providers
#[async_trait::async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat(
        &self,
        messages: &[Value],
        model: &str,
        tools: &[Value],
    ) -> Result<super::agent::LLMResponse, String>;
    fn name(&self) -> &str;
    fn api_base(&self) -> &str;
}

/// OpenRouter provider
pub struct OpenRouterProvider {
    api_key: String,
    api_base: String,
}

impl OpenRouterProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            api_base: "https://openrouter.ai/api/v1".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for OpenRouterProvider {
    async fn chat(
        &self,
        messages: &[Value],
        model: &str,
        tools: &[Value],
    ) -> Result<super::agent::LLMResponse, String> {
        let client = reqwest::Client::new();

        let body = json!({
            "model": model,
            "messages": messages,
            "tools": tools,
            "tool_choice": if tools.is_empty() { json!(null) } else { json!("auto") }
        });

        let response = client
            .post(&self.api_base)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://github.com/HKUDS/nanobot")
            .header("X-Title", "nanobot")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("API error: {}", error));
        }

        parse_llm_response(response).await
    }

    fn name(&self) -> &str {
        "openrouter"
    }

    fn api_base(&self) -> &str {
        &self.api_base
    }
}

/// Anthropic (Claude) provider
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
    ) -> Result<super::agent::LLMResponse, String> {
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

        parse_anthropic_response(response).await
    }

    fn name(&self) -> &str {
        "anthropic"
    }

    fn api_base(&self) -> &str {
        &self.api_base
    }
}

/// OpenAI provider (GPT-4, GPT-3.5)
pub struct OpenAIProvider {
    api_key: String,
    api_base: String,
}

impl OpenAIProvider {
    pub fn new(api_key: String, api_base: Option<String>) -> Self {
        Self {
            api_key,
            api_base: api_base.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for OpenAIProvider {
    async fn chat(
        &self,
        messages: &[Value],
        model: &str,
        tools: &[Value],
    ) -> Result<super::agent::LLMResponse, String> {
        let client = reqwest::Client::new();

        let body = json!({
            "model": model,
            "messages": messages,
            "tools": tools,
            "tool_choice": if tools.is_empty() { json!(null) } else { json!("auto") }
        });

        let response = client
            .post(&format!("{}/chat/completions", self.api_base))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("API error: {}", error));
        }

        parse_llm_response(response).await
    }

    fn name(&self) -> &str {
        "openai"
    }

    fn api_base(&self) -> &str {
        &self.api_base
    }
}

/// Groq provider (fast inference)
pub struct GroqProvider {
    api_key: String,
    api_base: String,
}

impl GroqProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            api_base: "https://api.groq.com/openai/v1".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for GroqProvider {
    async fn chat(
        &self,
        messages: &[Value],
        model: &str,
        tools: &[Value],
    ) -> Result<super::agent::LLMResponse, String> {
        let client = reqwest::Client::new();

        let body = json!({
            "model": model,
            "messages": messages,
            "tools": tools,
            "tool_choice": if tools.is_empty() { json!(null) } else { json!("auto") }
        });

        let response = client
            .post(&format!("{}/chat/completions", self.api_base))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(format!("API error: {}", error));
        }

        parse_llm_response(response).await
    }

    fn name(&self) -> &str {
        "groq"
    }

    fn api_base(&self) -> &str {
        &self.api_base
    }
}

/// Gemini provider (Google)
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
    ) -> Result<super::agent::LLMResponse, String> {
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

        parse_gemini_response(response).await
    }

    fn name(&self) -> &str {
        "gemini"
    }

    fn api_base(&self) -> &str {
        &self.api_base
    }
}

/// MiniMax provider
pub struct MiniMaxProvider {
    api_key: String,
    api_base: String,
}

impl MiniMaxProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            api_base: "https://api.minimax.chat/v1".to_string(),
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
    ) -> Result<super::agent::LLMResponse, String> {
        let client = reqwest::Client::new();

        // MiniMax expects model ID without provider prefix (e.g., "minimax-2.1" not "minimax/minimax-2.1")
        let model_id = model.split('/').last().unwrap_or(model);

        // MiniMax uses OpenAI-compatible API
        let body = json!({
            "model": model_id,
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

        let status = response.status();
        let text = response.text().await.map_err(|e| format!("Read error: {}", e))?;

        if !status.is_success() {
            return Err(format!("API error (status {}): {}", status, text));
        }

        // Parse the text as JSON
        let response_json: Value = serde_json::from_str(&text)
            .map_err(|e| format!("Parse error: {}", e))?;

        let choice = &response_json["choices"][0];
        let content = choice["message"]["content"].as_str().map(|s| s.to_string());

        let tool_calls: Vec<super::agent::ToolCall> = if let Some(tc_array) = choice["message"]["tool_calls"].as_array() {
            tc_array.iter().map(|tc| super::agent::ToolCall {
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

        Ok(super::agent::LLMResponse {
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

/// Provider factory
pub fn create_provider(config: &Config) -> Box<dyn LLMProvider> {
    // Priority: OpenRouter > Anthropic > OpenAI > Groq > Gemini > MiniMax
    if !config.providers.openrouter.api_key.is_empty() {
        Box::new(OpenRouterProvider::new(config.providers.openrouter.api_key.clone()))
    } else if !config.providers.anthropic.api_key.is_empty() {
        Box::new(AnthropicProvider::new(config.providers.anthropic.api_key.clone()))
    } else if !config.providers.openai.api_key.is_empty() {
        Box::new(OpenAIProvider::new(
            config.providers.openai.api_key.clone(),
            config.providers.openai.api_base.clone()
        ))
    } else if !config.providers.groq.api_key.is_empty() {
        Box::new(GroqProvider::new(config.providers.groq.api_key.clone()))
    } else if !config.providers.gemini.api_key.is_empty() {
        Box::new(GeminiProvider::new(config.providers.gemini.api_key.clone()))
    } else if !config.providers.minimax.api_key.is_empty() {
        Box::new(MiniMaxProvider::new(config.providers.minimax.api_key.clone()))
    } else {
        Box::new(OpenRouterProvider::new(String::new()))
    }
}

/// Parse OpenAI-compatible response
async fn parse_llm_response(response: reqwest::Response) -> Result<super::agent::LLMResponse, String> {
    let response_json: Value = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let choice = &response_json["choices"][0];
    let content = choice["message"]["content"].as_str().map(|s| s.to_string());

    let tool_calls: Vec<super::agent::ToolCall> = if let Some(tc_array) = choice["message"]["tool_calls"].as_array() {
        tc_array.iter().map(|tc| super::agent::ToolCall {
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

    Ok(super::agent::LLMResponse {
        content,
        tool_calls,
        finish_reason,
    })
}

/// Parse Anthropic response
async fn parse_anthropic_response(response: reqwest::Response) -> Result<super::agent::LLMResponse, String> {
    let response_json: Value = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let content = response_json["content"][0]["text"].as_str()
        .map(|s| s.to_string());

    let tool_calls: Vec<super::agent::ToolCall> = if let Some(tc_array) = response_json["content"].as_array() {
        tc_array.iter()
            .filter(|tc| tc["type"] == "tool_use")
            .map(|tc| super::agent::ToolCall {
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

    Ok(super::agent::LLMResponse {
        content,
        tool_calls,
        finish_reason,
    })
}

/// Parse Gemini response
async fn parse_gemini_response(response: reqwest::Response) -> Result<super::agent::LLMResponse, String> {
    let response_json: Value = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let content = response_json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_default();

    // Gemini doesn't support tool calls in the same way
    Ok(super::agent::LLMResponse {
        content: Some(content),
        tool_calls: vec![],
        finish_reason: "stop".to_string(),
    })
}
