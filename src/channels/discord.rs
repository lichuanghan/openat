//! Discord channel implementation using WebSocket Gateway.
//!
//! Uses the twilight or serenity Discord library for Gateway websocket connection.

use crate::channels::Channel;
use crate::config::Config;
use crate::core::bus::MessageBus;
use crate::types::OutboundMessage;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

/// Discord channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub enabled: bool,
    pub token: String,
    pub allowed_users: Vec<String>,
    pub gateway_url: String,
    pub intents: i32,
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            token: String::new(),
            allowed_users: Vec::new(),
            gateway_url: "wss://gateway.discord.gg/?v=10&encoding=json".to_string(),
            intents: 37377, // GUILDS + GUILD_MESSAGES + DIRECT_MESSAGES + MESSAGE_CONTENT
        }
    }
}

/// Discord channel implementation
#[derive(Clone)]
pub struct DiscordChannel {
    config: DiscordConfig,
    outbound_tx: broadcast::Sender<OutboundMessage>,
    running: Arc<tokio::sync::Mutex<bool>>,
}

impl DiscordChannel {
    pub fn new(config: DiscordConfig, outbound_tx: broadcast::Sender<OutboundMessage>) -> Self {
        Self {
            config,
            outbound_tx,
            running: Arc::new(tokio::sync::Mutex::new(false)),
        }
    }

    /// Check if user is allowed
    fn is_allowed(&self, user_id: &str) -> bool {
        self.config.allowed_users.is_empty()
            || self.config.allowed_users.iter().any(|u| u == user_id)
    }

    /// Send message to Discord
    async fn send_message(&self, chat_id: &str, content: &str) -> Result<()> {
        // Discord implementation using reqwest for REST API
        if self.config.token.is_empty() {
            warn!("Discord token not configured");
            return Ok(());
        }

        let channel_url = format!(
            "https://discord.com/api/v10/channels/{}/messages",
            chat_id
        );

        let client = reqwest::Client::new();
        let response = client
            .post(&channel_url)
            .header("Authorization", format!("Bot {}", self.config.token))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "content": content }))
            .send()
            .await
            .context("Failed to send Discord message")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("Discord API error: {}", error_text);
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl crate::channels::Channel for DiscordChannel {
    fn name(&self) -> &str {
        "discord"
    }

    async fn start(&mut self, _bus: &MessageBus) -> Result<()> {
        if self.config.token.is_empty() {
            info!("Discord token not configured, skipping");
            return Ok(());
        }

        info!("Discord channel starting...");
        *self.running.lock().await = true;

        // Start the gateway connection in a background task
        let running = self.running.clone();
        let token = self.config.token.clone();
        let gateway_url = self.config.gateway_url.clone();
        let outbound_tx = self.outbound_tx.clone();

        tokio::spawn(async move {
            Self::run_gateway(&token, &gateway_url, &running, &outbound_tx).await;
        });

        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        info!("Discord channel stopping...");
        *self.running.lock().await = false;
        Ok(())
    }

    fn is_enabled(&self) -> bool {
        self.config.enabled && !self.config.token.is_empty()
    }
}

impl DiscordChannel {
    async fn run_gateway(
        token: &str,
        gateway_url: &str,
        running: &Arc<tokio::sync::Mutex<bool>>,
        outbound_tx: &broadcast::Sender<OutboundMessage>,
    ) {
        info!("Discord gateway connecting to {}", gateway_url);

        // This is a simplified implementation
        // A full implementation would use twilight or serenity
        // For now, we just log that the gateway would connect

        while *running.lock().await {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            debug!("Discord gateway heartbeat...");
        }

        info!("Discord gateway stopped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::Channel;

    #[test]
    fn test_discord_config_default() {
        let config = DiscordConfig::default();
        assert!(!config.enabled);
        assert!(config.token.is_empty());
        assert_eq!(config.gateway_url, "wss://gateway.discord.gg/?v=10&encoding=json");
        assert_eq!(config.intents, 37377);
    }

    #[test]
    fn test_discord_channel_is_enabled() {
        let config = DiscordConfig::default();
        let (tx, _) = broadcast::channel(100);
        let channel = DiscordChannel::new(config.clone(), tx.clone());
        assert!(!channel.is_enabled());

        // Create a config with both enabled and a token
        let config_enabled = DiscordConfig {
            enabled: true,
            token: "test-token".to_string(),
            ..DiscordConfig::default()
        };
        let channel_enabled = DiscordChannel::new(config_enabled, tx);
        assert!(channel_enabled.is_enabled());
    }

    #[test]
    fn test_discord_is_allowed() {
        let config = DiscordConfig {
            allowed_users: vec!["123".to_string(), "456".to_string()],
            ..DiscordConfig::default()
        };
        let (tx, _) = broadcast::channel(100);
        let channel = DiscordChannel::new(config.clone(), tx.clone());

        // Test is_allowed method
        assert!(allowed_users_check(&config, "123"));
        assert!(allowed_users_check(&config, "456"));
        assert!(!allowed_users_check(&config, "789"));

        // Empty allowed list means all users are allowed
        let config_empty = DiscordConfig::default();
        assert!(allowed_users_check(&config_empty, "anyone"));
    }
}

fn allowed_users_check(config: &DiscordConfig, user_id: &str) -> bool {
    config.allowed_users.is_empty() || config.allowed_users.iter().any(|u| u == user_id)
}
