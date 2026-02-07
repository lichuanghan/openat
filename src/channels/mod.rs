pub mod discord;
pub mod feishu;
pub mod telegram;
pub mod whatsapp;
pub mod qq;

use crate::core::bus::MessageBus;
use crate::config::Config;
use crate::types::OutboundMessage;
use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::broadcast;
use tracing::info;

/// Trait for channel implementations
#[async_trait::async_trait]
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;
    async fn start(&mut self, bus: &MessageBus) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
    fn is_enabled(&self) -> bool;
}

/// Channel manager that coordinates all channels
pub struct ChannelManager {
    bus: MessageBus,
    outbound_tx: broadcast::Sender<OutboundMessage>,
    channels: HashMap<String, Box<dyn Channel>>,
}

impl ChannelManager {
    pub fn new() -> Self {
        let (outbound_tx, _) = broadcast::channel(100);
        Self {
            bus: MessageBus::new(),
            outbound_tx,
            channels: HashMap::new(),
        }
    }

    /// Get the message bus
    pub fn bus(&self) -> &MessageBus {
        &self.bus
    }

    /// Subscribe to outbound messages
    pub fn subscribe_outbound(&self) -> broadcast::Receiver<OutboundMessage> {
        self.outbound_tx.subscribe()
    }

    /// Publish an outbound message
    pub async fn publish_outbound(&self, msg: OutboundMessage) {
        let _ = self.outbound_tx.send(msg);
    }

    /// Load and initialize channels from config
    pub async fn initialize(&mut self, config: &Config) -> Result<()> {
        // Initialize Telegram
        if config.channels.telegram.enabled && !config.channels.telegram.token.is_empty() {
            info!("Initializing Telegram channel...");
            // Telegram is started separately in the gateway
        }

        // Initialize WhatsApp
        if config.channels.whatsapp.enabled {
            info!("Initializing WhatsApp channel...");
            // WhatsApp bridge would be started here
        }

        Ok(())
    }

    /// Start all enabled channels
    pub async fn start(&mut self, config: &Config) -> Result<()> {
        self.initialize(config).await?;
        Ok(())
    }

    /// Stop all channels
    pub async fn stop(&mut self) {
        for (name, channel) in &mut self.channels {
            let _ = channel.stop().await;
            info!("Stopped channel: {}", name);
        }
        self.channels.clear();
    }
}

impl Default for ChannelManager {
    fn default() -> Self {
        Self::new()
    }
}
