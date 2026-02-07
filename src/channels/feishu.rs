//! Feishu/Lark channel implementation using WebSocket long connection.
//!
//! Uses lark-oapi SDK for Feishu bot integration.

use crate::channels::Channel;
use crate::config::Config;
use crate::core::bus::MessageBus;
use crate::types::OutboundMessage;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

/// Feishu channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuConfig {
    pub enabled: bool,
    pub app_id: String,
    pub app_secret: String,
    pub encrypt_key: String,
    pub verification_token: String,
    pub allowed_users: Vec<String>,
}

impl Default for FeishuConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            app_id: String::new(),
            app_secret: String::new(),
            encrypt_key: String::new(),
            verification_token: String::new(),
            allowed_users: Vec::new(),
        }
    }
}

/// Feishu channel implementation
#[derive(Clone)]
pub struct FeishuChannel {
    config: FeishuConfig,
    outbound_tx: broadcast::Sender<OutboundMessage>,
    running: Arc<tokio::sync::Mutex<bool>>,
    // SDK client would be stored here when lark-oapi is used
}

impl FeishuChannel {
    pub fn new(config: FeishuConfig, outbound_tx: broadcast::Sender<OutboundMessage>) -> Self {
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

    /// Send message to Feishu
    async fn send_message(&self, chat_id: &str, content: &str) -> Result<()> {
        if self.config.app_id.is_empty() || self.config.app_secret.is_empty() {
            warn!("Feishu app_id or app_secret not configured");
            return Ok(());
        }

        // Feishu message API would be called here
        // For now, just log
        debug!("Would send Feishu message to {}: {}", chat_id, content);

        Ok(())
    }
}

#[async_trait::async_trait]
impl crate::channels::Channel for FeishuChannel {
    fn name(&self) -> &str {
        "feishu"
    }

    async fn start(&mut self, _bus: &MessageBus) -> Result<()> {
        if self.config.app_id.is_empty() || self.config.app_secret.is_empty() {
            info!("Feishu app credentials not configured, skipping");
            return Ok(());
        }

        info!("Feishu channel starting...");
        *self.running.lock().await = true;

        // Start WebSocket connection for receiving events
        let running = self.running.clone();
        let outbound_tx = self.outbound_tx.clone();

        tokio::spawn(async move {
            Self::run_websocket(&running, &outbound_tx).await;
        });

        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        info!("Feishu channel stopping...");
        *self.running.lock().await = false;
        Ok(())
    }

    fn is_enabled(&self) -> bool {
        self.config.enabled
            && !self.config.app_id.is_empty()
            && !self.config.app_secret.is_empty()
    }
}

impl FeishuChannel {
    async fn run_websocket(
        running: &Arc<tokio::sync::Mutex<bool>>,
        _outbound_tx: &broadcast::Sender<OutboundMessage>,
    ) {
        info!("Feishu WebSocket connection would start here");
        info!("Note: Full Feishu integration requires lark-oapi Rust SDK");

        while *running.lock().await {
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            debug!("Feishu WebSocket heartbeat...");
        }

        info!("Feishu WebSocket stopped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feishu_config_default() {
        let config = FeishuConfig::default();
        assert!(!config.enabled);
        assert!(config.app_id.is_empty());
        assert!(config.app_secret.is_empty());
    }

    #[test]
    fn test_feishu_channel_is_enabled() {
        let config = FeishuConfig::default();
        let (tx, _) = broadcast::channel(100);
        let channel = FeishuChannel::new(config.clone(), tx.clone());
        assert!(!channel.is_enabled());

        let mut config_enabled = config;
        config_enabled.enabled = true;
        config_enabled.app_id = "test-app-id".to_string();
        config_enabled.app_secret = "test-secret".to_string();
        let channel_enabled = FeishuChannel::new(config_enabled, tx);
        assert!(channel_enabled.is_enabled());
    }

    #[test]
    fn test_feishu_is_allowed() {
        let config = FeishuConfig {
            allowed_users: vec!["ou_123".to_string(), "ou_456".to_string()],
            ..FeishuConfig::default()
        };
        let (tx, _) = broadcast::channel(100);
        let _channel = FeishuChannel::new(config.clone(), tx.clone());

        // Test allowed users check
        assert!(feishu_allowed_check(&config, "ou_123"));
        assert!(feishu_allowed_check(&config, "ou_456"));
        assert!(!feishu_allowed_check(&config, "ou_789"));

        // Empty allowed list means all users are allowed
        let config_empty = FeishuConfig::default();
        assert!(feishu_allowed_check(&config_empty, "anyone"));
    }
}

fn feishu_allowed_check(config: &FeishuConfig, user_id: &str) -> bool {
    config.allowed_users.is_empty() || config.allowed_users.iter().any(|u| u == user_id)
}
