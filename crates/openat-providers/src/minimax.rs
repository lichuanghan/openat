//! MiniMax provider - Chinese LLM.

use crate::{LLMProvider, LLMResponse, StreamResponse};
use super::openai_compat::OpenAICompatConfig;
use serde_json::Value;
use std::pin::Pin;
use futures_util::Stream;

/// MiniMax provider
#[derive(Debug, Clone)]
pub struct MiniMaxProvider {
    config: OpenAICompatConfig,
}

impl MiniMaxProvider {
    pub fn new(api_key: String, api_base: Option<String>) -> Self {
        let base = api_base.unwrap_or_else(|| "https://api.minimax.chat/v1".to_string());
        Self {
            config: OpenAICompatConfig::new(
                api_key,
                base,
                "minimax",
            ),
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
        self.config.chat_impl(messages, model, tools).await
    }

    fn stream(
        &self,
        messages: &[Value],
        model: &str,
        tools: &[Value],
    ) -> Pin<Box<dyn Stream<Item = Result<StreamResponse, String>> + Send>> {
        self.config.stream_impl(messages, model, tools)
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        self.config.name
    }

    fn api_base(&self) -> &str {
        &self.config.api_base
    }
}
