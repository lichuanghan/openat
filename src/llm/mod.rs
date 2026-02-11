//! LLM module - providers for various LLM services.
//!
//! # Supported Providers
//!
//! - OpenRouter (OpenAI-compatible)
//! - Anthropic (Claude)
//! - OpenAI (GPT-4, GPT-3.5)
//! - Groq (Fast inference)
//! - Gemini (Google)
//! - MiniMax (Chinese LLM)

pub mod providers;

pub use providers::{
    create_provider, AnthropicProvider, GeminiProvider, GroqProvider,
    LLMProvider, MiniMaxProvider, OpenAIProvider, OpenRouterProvider,
};
