//! Discord channel implementation using WebSocket Gateway.
//!
//! This module provides a complete Discord bot implementation that connects
//! to Discord's Gateway via WebSocket and handles messages.

use crate::channels::Channel;
use crate::config::Discord as DiscordConfig;
use crate::core::bus::MessageBus;
use crate::types::{InboundMessage, OutboundMessage};
use anyhow::{Context, Result};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use tokio::time::{interval, sleep, Duration};
use tracing::{debug, error, info, warn};
use tokio_tungstenite::tungstenite::protocol::Message;

/// Type alias for WebSocket sender
type WsSender = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    Message,
>;

/// Discord Gateway opcodes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OpCode {
    Dispatch = 0,
    Heartbeat = 1,
    Identify = 2,
    Resume = 6,
    Reconnect = 7,
    InvalidSession = 9,
    Hello = 10,
    HeartbeatAck = 11,
}

impl OpCode {
    pub fn from_i64(n: i64) -> Option<Self> {
        match n {
            0 => Some(OpCode::Dispatch),
            1 => Some(OpCode::Heartbeat),
            2 => Some(OpCode::Identify),
            6 => Some(OpCode::Resume),
            7 => Some(OpCode::Reconnect),
            9 => Some(OpCode::InvalidSession),
            10 => Some(OpCode::Hello),
            11 => Some(OpCode::HeartbeatAck),
            _ => None,
        }
    }
}

/// Gateway Hello event
#[derive(Debug, Deserialize)]
struct HelloPayload {
    heartbeat_interval: u64,
}

/// Gateway message structure
#[derive(Debug, Deserialize)]
struct GatewayMessage {
    op: i64,
    #[serde(default)]
    t: Option<String>,
    #[serde(default)]
    s: Option<u64>,
    #[serde(default)]
    d: Option<serde_json::Value>,
}

/// Discord channel implementation with full Gateway support
#[derive(Clone)]
pub struct DiscordChannel {
    config: DiscordConfig,
    running: Arc<Mutex<bool>>,
    session_id: Arc<Mutex<Option<String>>>,
    sequence: Arc<Mutex<Option<u64>>>,
    heartbeat_interval: Arc<Mutex<u64>>,
}

impl DiscordChannel {
    /// Create a new Discord channel
    pub fn new(config: DiscordConfig) -> Self {
        Self {
            config,
            running: Arc::new(Mutex::new(false)),
            session_id: Arc::new(Mutex::new(None)),
            sequence: Arc::new(Mutex::new(None)),
            heartbeat_interval: Arc::new(Mutex::new(0)),
        }
    }

    /// Check if user is allowed
    fn is_allowed(&self, user_id: &str) -> bool {
        self.config.allowed_users.is_empty()
            || self.config.allowed_users.iter().any(|u| u == user_id)
    }

    /// Connect to Gateway and handle events
    pub async fn connect_gateway(&self, bus: &MessageBus) {
        let mut reconnect_delay = 1u64;

        loop {
            if !*self.running.lock().await {
                break;
            }

            info!("Connecting to Discord Gateway...");

            // Get gateway URL from Discord API
            let gateway_url = match self.get_gateway_url().await {
                Ok(url) => url,
                Err(e) => {
                    error!("Failed to get gateway URL: {}", e);
                    break;
                }
            };

            info!("Gateway URL: {}", gateway_url);

            // Connect to WebSocket
            let (ws_stream, _) = match tokio_tungstenite::connect_async(&gateway_url).await {
                Ok(stream) => {
                    info!("WebSocket handshake successful");
                    stream
                }
                Err(e) => {
                    error!("Failed to connect to Gateway: {}", e);
                    info!("Reconnecting in {} seconds...", reconnect_delay);
                    sleep(Duration::from_secs(reconnect_delay)).await;
                    reconnect_delay = (reconnect_delay * 2).min(60);
                    continue;
                }
            };

            let (ws_sender, mut ws_receiver) = ws_stream.split();
            *self.running.lock().await = true;
            reconnect_delay = 1; // Reset delay on successful connection
            info!("Gateway connected, starting event loop...");

            // Reset heartbeat interval until we receive Hello
            *self.heartbeat_interval.lock().await = 0;

            // Wrap sender in Arc<Mutex> to share between tasks
            let mut ws_sender = Arc::new(Mutex::new(ws_sender));

            // Main event loop with heartbeat support
            let heartbeat_interval_mut = self.heartbeat_interval.clone();
            let running_clone = self.running.clone();
            let sequence_clone = self.sequence.clone();
            let ws_sender_clone = ws_sender.clone();

            // Spawn heartbeat task that uses the same WebSocket connection
            let heartbeat_task = tokio::spawn(async move {
                let mut interval_ms = 0u64;
                let mut heartbeat_interval = interval(Duration::from_millis(41250)); // Default, will be updated

                loop {
                    // Check if we have a valid heartbeat interval
                    let new_interval = *heartbeat_interval_mut.lock().await;
                    if new_interval > 0 && new_interval != interval_ms {
                        interval_ms = new_interval;
                        heartbeat_interval = interval(Duration::from_millis(new_interval));
                        info!("Heartbeat interval updated to {}ms", new_interval);
                    }

                    // Check if we should stop
                    if !*running_clone.lock().await {
                        break;
                    }

                    // Wait for next heartbeat tick
                    heartbeat_interval.tick().await;

                    // Check if we should stop again (in case it changed during tick)
                    if !*running_clone.lock().await {
                        break;
                    }

                    // Send heartbeat on existing connection
                    let seq = *sequence_clone.lock().await;
                    let heartbeat = json!({
                        "op": OpCode::Heartbeat as i64,
                        "d": seq
                    });

                    let mut sender = ws_sender_clone.lock().await;
                    if let Err(e) = sender.send(Message::Text(heartbeat.to_string())).await {
                        error!("Failed to send heartbeat: {}", e);
                        break; // Exit heartbeat loop on error
                    } else {
                        debug!("Sent heartbeat");
                    }
                }
            });

            // Process incoming messages
            while *self.running.lock().await {
                // Process incoming messages
                if let Some(msg_result) = ws_receiver.next().await {
                    match msg_result {
                        Ok(Message::Text(text)) => {
                            debug!("Received: {}", text);
                            if let Ok(gateway_msg) = serde_json::from_str::<GatewayMessage>(&text) {
                                if let Some(event_type) = &gateway_msg.t {
                                    info!("Event: {} (op={})", event_type, gateway_msg.op);
                                }
                                self.handle_gateway_message(&gateway_msg, &mut ws_sender, bus).await;

                                // If Hello was received, heartbeat task should now have valid interval
                                if let Ok(gateway_msg) = serde_json::from_str::<GatewayMessage>(&text) {
                                    if let Some(op) = OpCode::from_i64(gateway_msg.op) {
                                        if op == OpCode::Hello {
                                            // Identify was already sent in handle_gateway_message
                                        }
                                    }
                                }
                            }
                        }
                        Ok(Message::Close(reason)) => {
                            info!("WebSocket closed: {:?}", reason);
                            break;
                        }
                        Err(e) => {
                            error!("WebSocket error: {}", e);
                            break;
                        }
                        _ => {}
                    }
                } else {
                    info!("WebSocket stream ended");
                    break;
                }
            }

            // Stop heartbeat task
            *self.running.lock().await = false;
            heartbeat_task.abort();
            let _ = heartbeat_task.await;

            info!("Gateway disconnected, will reconnect in {} seconds...", reconnect_delay);
            sleep(Duration::from_secs(reconnect_delay)).await;
            reconnect_delay = (reconnect_delay * 2).min(60);
        }

        info!("Discord gateway connection closed");
    }

    /// Handle incoming Gateway message
    async fn handle_gateway_message(
        &self,
        msg: &GatewayMessage,
        ws_sender: &mut Arc<Mutex<WsSender>>,
        bus: &MessageBus,
    ) {
        // Update sequence number
        if let Some(s) = msg.s {
            *self.sequence.lock().await = Some(s);
        }

        if let Some(op) = OpCode::from_i64(msg.op) {
            match op {
                OpCode::Hello => {
                    if let Some(d) = &msg.d {
                        if let Ok(payload) = serde_json::from_value::<HelloPayload>(d.clone()) {
                            *self.heartbeat_interval.lock().await = payload.heartbeat_interval;
                            info!(
                                "Received Hello, heartbeat interval: {}ms",
                                payload.heartbeat_interval
                            );

                            // Send Identify
                            self.identify(ws_sender).await;
                        }
                    }
                }
                OpCode::HeartbeatAck => {
                    debug!("Received Heartbeat ACK");
                }
                OpCode::Dispatch => {
                    if let Some(event_type) = &msg.t {
                        match event_type.as_str() {
                            "READY" => {
                                info!("Discord Gateway ready!");
                                if let Some(d) = &msg.d {
                                    if let Some(session) = d.get("session_id") {
                                        if let Some(sid) = session.as_str() {
                                            *self.session_id.lock().await = Some(sid.to_string());
                                            info!("Session ID: {}", sid);
                                        }
                                    }
                                }
                            }
                            "RESUMED" => {
                                info!("Discord session resumed");
                            }
                            "MESSAGE_CREATE" => {
                                if let Some(d) = &msg.d {
                                    self.handle_message(d, bus).await;
                                }
                            }
                            "INVALID_SESSION" => {
                                warn!("Invalid session received");
                                if let Some(d) = &msg.d {
                                    error!("Session error data: {}", d);
                                }
                                *self.session_id.lock().await = None;
                            }
                            _ => {
                                debug!("Discord event: {}", event_type);
                            }
                        }
                    }
                }
                OpCode::Heartbeat => {
                    let heartbeat = json!({
                        "op": OpCode::Heartbeat as i64,
                        "d": *self.sequence.lock().await
                    });
                    let mut sender = ws_sender.lock().await;
                    let _ = sender.send(Message::Text(heartbeat.to_string())).await;
                }
                OpCode::Reconnect => {
                    info!("Reconnect requested by Discord");
                    *self.running.lock().await = false;
                }
                OpCode::Identify => {}
                OpCode::Resume => {}
                OpCode::InvalidSession => {
                    warn!("Invalid session");
                    *self.session_id.lock().await = None;
                }
            }
        }
    }

    /// Send Identify payload
    async fn identify(
        &self,
        ws_sender: &mut Arc<Mutex<WsSender>>,
    ) {
        let mut properties = HashMap::new();
        properties.insert("os".to_string(), "linux".to_string());
        properties.insert("browser".to_string(), "openat".to_string());
        properties.insert("device".to_string(), "openat".to_string());

        let identify = json!({
            "op": OpCode::Identify as i64,
            "d": {
                "token": self.config.token,
                "properties": properties,
                "intents": self.config.intents
            }
        });

        let mut sender = ws_sender.lock().await;
        if let Err(e) = sender.send(Message::Text(identify.to_string())).await {
            error!("Failed to send Identify: {}", e);
        } else {
            info!("Sent Identify");
        }
    }

    /// Get gateway URL from Discord API
    async fn get_gateway_url(&self) -> Result<String> {
        match Self::get_gateway_url_internal(&self.config.token).await {
            Ok(Some(url)) => Ok(url),
            Ok(None) => anyhow::bail!("Failed to get gateway URL"),
            Err(e) => Err(e),
        }
    }

    /// Internal helper to get gateway URL
    async fn get_gateway_url_internal(token: &str) -> Result<Option<String>> {
        #[derive(Debug, Deserialize)]
        struct GatewayInfoResponse {
            url: String,
        }

        let client = reqwest::Client::new();
        let response = match client
            .get("https://discord.com/api/v10/gateway/bot")
            .header("Authorization", format!("Bot {}", token))
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return Ok(None),
        };

        if !response.status().is_success() {
            return Ok(None);
        }

        let text = response.text().await?;
        match serde_json::from_str::<GatewayInfoResponse>(&text) {
            // Discord expects: wss://gateway.discord.gg/?v=10&encoding=json
            Ok(info) => Ok(Some(format!("{}/?v=10&encoding=json", info.url))),
            Err(_) => Ok(None),
        }
    }

    /// Handle incoming message
    async fn handle_message(&self, message: &serde_json::Value, bus: &MessageBus) {
        // Debug: log full message
        info!("Raw message: {}", message);

        // Skip bot messages
        if let Some(author) = message.get("author") {
            if let Some(bot) = author.get("bot") {
                if bot.as_bool().unwrap_or(false) {
                    info!("Skipping bot message");
                    return;
                }
            }
        }

        // Get content and clean up mention prefix
        let mut content = message.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();

        // Remove @mention prefix (e.g., "<@123456789> " -> "")
        if content.starts_with("<@") {
            if let Some(end) = content.find(">") {
                content = content[end + 1..].trim_start().to_string();
            }
        }

        info!("Cleaned message content: '{}'", content);

        if content.is_empty() {
            info!("Empty content, skipping");
            return;
        }

        let channel_id = message.get("channel_id").and_then(|v| v.as_str()).unwrap_or("");
        let sender_id = message.get("author").and_then(|a| a.get("id")).and_then(|v| v.as_str()).unwrap_or("");

        info!("Channel: {}, Sender: {}", channel_id, sender_id);

        // Check if user is allowed
        if !self.is_allowed(sender_id) {
            debug!("User {} not in allowed list", sender_id);
            return;
        }

        // Send immediate acknowledgment to prevent Discord timeout
        let _ = self.send_message(channel_id, "正在思考...").await;

        let inbound = InboundMessage {
            channel: "discord".to_string(),
            sender_id: sender_id.to_string(),
            chat_id: channel_id.to_string(),
            content: content.to_string(),
            timestamp: Utc::now(),
            media: vec![],
            metadata: HashMap::new(),
        };

        info!("Discord message from {}: {}", sender_id, content);
        bus.publish_inbound(inbound).await;
    }

    /// Send a message to Discord channel via REST API
    async fn send_message(&self, channel_id: &str, content: &str) -> Result<()> {
        if self.config.token.is_empty() {
            warn!("Discord token not configured");
            return Ok(());
        }

        let client = reqwest::Client::new();
        let response = client
            .post(&format!(
                "https://discord.com/api/v10/channels/{}/messages",
                channel_id
            ))
            .header("Authorization", format!("Bot {}", self.config.token))
            .header("Content-Type", "application/json")
            .json(&json!({ "content": content }))
            .send()
            .await
            .context("Failed to send Discord message")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("Discord API error: {}", error_text);
            anyhow::bail!("Discord API error: {}", error_text);
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl Channel for DiscordChannel {
    fn name(&self) -> &str {
        "discord"
    }

    async fn start(&mut self, bus: &crate::core::bus::MessageBus) -> Result<()> {
        if !self.config.enabled || self.config.token.is_empty() {
            info!("Discord not enabled or token not configured, skipping");
            return Ok(());
        }

        info!("Discord channel starting...");
        *self.running.lock().await = true;

        let channel = self.clone();
        let bus_for_gateway = bus.clone();

        // Start gateway connection in background
        tokio::spawn(async move {
            channel.connect_gateway(&bus_for_gateway).await;
        });

        // Start outbound message handler - subscribe to MessageBus outbound channel
        let outbound_rx = bus.subscribe_outbound();
        let config = self.config.clone();
        tokio::spawn(async move {
            Self::handle_outbound_messages(outbound_rx, &config).await;
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
    /// Handle outbound messages
    async fn handle_outbound_messages(
        mut rx: broadcast::Receiver<OutboundMessage>,
        config: &DiscordConfig,
    ) {
        while let Ok(msg) = rx.recv().await {
            if msg.channel == "discord" {
                if let Err(e) = Self::send_message_impl(config, &msg.chat_id, &msg.content).await {
                    error!("Failed to send outbound message: {}", e);
                }
            }
        }
    }

    async fn send_message_impl(config: &DiscordConfig, channel_id: &str, content: &str) -> Result<()> {
        if config.token.is_empty() {
            warn!("Discord token not configured");
            return Ok(());
        }

        let client = reqwest::Client::new();
        let response = client
            .post(&format!(
                "https://discord.com/api/v10/channels/{}/messages",
                channel_id
            ))
            .header("Authorization", format!("Bot {}", config.token))
            .header("Content-Type", "application/json")
            .json(&json!({ "content": content }))
            .send()
            .await
            .context("Failed to send Discord message")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("Discord API error: {}", error_text);
            anyhow::bail!("Discord API error: {}", error_text);
        }

        Ok(())
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
        let channel = DiscordChannel::new(config.clone());
        assert!(!channel.is_enabled());

        let config_enabled = DiscordConfig {
            enabled: true,
            token: "test-token".to_string(),
            ..DiscordConfig::default()
        };
        let channel_enabled = DiscordChannel::new(config_enabled);
        assert!(channel_enabled.is_enabled());
    }

    #[test]
    fn test_discord_is_allowed() {
        let config = DiscordConfig {
            allowed_users: vec!["123".to_string(), "456".to_string()],
            ..DiscordConfig::default()
        };
        let channel = DiscordChannel::new(config.clone());

        assert!(allowed_users_check(&config, "123"));
        assert!(allowed_users_check(&config, "456"));
        assert!(!allowed_users_check(&config, "789"));

        let config_empty = DiscordConfig::default();
        assert!(allowed_users_check(&config_empty, "anyone"));
    }

    #[test]
    fn test_opcode_from_i64() {
        assert_eq!(OpCode::from_i64(0), Some(OpCode::Dispatch));
        assert_eq!(OpCode::from_i64(1), Some(OpCode::Heartbeat));
        assert_eq!(OpCode::from_i64(2), Some(OpCode::Identify));
        assert_eq!(OpCode::from_i64(10), Some(OpCode::Hello));
        assert_eq!(OpCode::from_i64(11), Some(OpCode::HeartbeatAck));
        assert_eq!(OpCode::from_i64(99), None);
    }
}

fn allowed_users_check(config: &DiscordConfig, user_id: &str) -> bool {
    config.allowed_users.is_empty() || config.allowed_users.iter().any(|u| u == user_id)
}
