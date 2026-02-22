//! Telegram channel adapter

use async_trait::async_trait;
use teloxide::prelude::*;
use tracing::info;

use openat_runtime::MessageBus;
use crate::Channel;

type ChannelResult<T = ()> = anyhow::Result<T>;

/// Telegram channel implementation
#[derive(Clone)]
pub struct TelegramChannel {
    bot: Option<Bot>,
}

impl TelegramChannel {
    pub fn new(bot: Option<Bot>) -> Self {
        Self { bot }
    }
}

#[async_trait]
impl Channel for TelegramChannel {
    fn name(&self) -> &str {
        "telegram"
    }

    async fn start(&mut self, _bus: &MessageBus) -> ChannelResult<()> {
        info!("Starting Telegram channel...");
        // Telegram channel requires teloxide's Dispatcher which has specific patterns
        // For now, the bot will need to be configured and started separately
        Ok(())
    }

    async fn stop(&mut self) -> ChannelResult<()> {
        self.bot = None;
        Ok(())
    }

    fn is_enabled(&self) -> bool {
        self.bot.is_some()
    }
}
