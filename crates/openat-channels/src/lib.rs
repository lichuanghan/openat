//! Channel adapters for openat
//!
//! Provides adapters for different messaging platforms:
//! Discord, Telegram, WhatsApp, QQ, etc.

use async_trait::async_trait;
use openat_runtime::MessageBus;

pub mod discord;
pub mod telegram;
pub mod qq;
pub mod common;

pub use discord::DiscordChannel;
pub use qq::QQChannel;

/// Trait for channel implementations
#[async_trait]
pub trait Channel: Send + Sync {
    /// Channel name
    fn name(&self) -> &str;

    /// Start the channel
    async fn start(&mut self, bus: &MessageBus) -> anyhow::Result<()>;

    /// Stop the channel
    async fn stop(&mut self) -> anyhow::Result<()>;

    /// Check if channel is enabled
    fn is_enabled(&self) -> bool;
}
