//! Discord channel adapter

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Duration;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use tracing::{error, info, warn};
use async_trait::async_trait;

use openat_runtime::MessageBus;
use openat_types::InboundMessage;
use crate::Channel;

type ChannelResult<T = ()> = anyhow::Result<T>;

use openat_config::Discord as DiscordConfig;

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

/// Discord channel implementation
#[derive(Clone)]
pub struct DiscordChannel {
    config: DiscordConfig,
    running: Arc<Mutex<bool>>,
    sequence: Arc<Mutex<Option<u64>>>,
}

impl DiscordChannel {
    pub fn new(config: DiscordConfig) -> Self {
        Self {
            config,
            running: Arc::new(Mutex::new(false)),
            sequence: Arc::new(Mutex::new(None)),
        }
    }

    fn is_allowed(&self, user_id: &str) -> bool {
        self.config.allowed_users.is_empty()
            || self.config.allowed_users.iter().any(|u| u == user_id)
    }

    /// Send a message to a Discord channel via REST API.
    /// Automatically splits messages longer than 2000 characters.
    /// Includes retry logic with exponential backoff for transient failures.
    async fn send_message(token: &str, channel_id: &str, content: &str) -> Result<(), String> {
        const MAX_RETRIES: u8 = 3;
        const BASE_DELAY_MS: u64 = 500;

        let client = reqwest::Client::new();
        let url = format!("https://discord.com/api/v10/channels/{}/messages", channel_id);

        // Discord limit is 2000 chars per message
        let chunks = split_message(content, 2000);

        for chunk in &chunks {
            let mut last_error = None;

            for attempt in 0..MAX_RETRIES {
                let resp = client
                    .post(&url)
                    .header("Authorization", format!("Bot {}", token))
                    .header("Content-Type", "application/json")
                    .json(&json!({ "content": chunk }))
                    .send()
                    .await;

                match resp {
                    Ok(resp) if resp.status().is_success() => {
                        // Success, break retry loop
                        last_error = None;
                        break;
                    }
                    Ok(resp) => {
                        let status = resp.status();
                        let body = resp.text().await.unwrap_or_default();

                        // Check if retryable (5xx errors, rate limiting 429)
                        if status.is_server_error() || status.as_u16() == 429 {
                            last_error = Some(format!("Discord API error {}: {}", status, body));
                            if attempt < MAX_RETRIES - 1 {
                                let delay = BASE_DELAY_MS * (2u64.pow(attempt as u32)).min(10000);
                                warn!("Discord send failed (attempt {}/{}), retrying after {}ms: {}",
                                    attempt + 1, MAX_RETRIES, delay, last_error.as_ref().unwrap());
                                tokio::time::sleep(Duration::from_millis(delay)).await;
                                continue;
                            }
                        } else {
                            // Non-retryable error
                            return Err(format!("Discord API error {}: {}", status, body));
                        }
                    }
                    Err(e) => {
                        // Network error - retry
                        last_error = Some(format!("Network error: {}", e));
                        if attempt < MAX_RETRIES - 1 {
                            let delay = BASE_DELAY_MS * (2u64.pow(attempt as u32)).min(10000);
                            warn!("Discord send failed (attempt {}/{}), retrying after {}ms: {}",
                                attempt + 1, MAX_RETRIES, delay, e);
                            tokio::time::sleep(Duration::from_millis(delay)).await;
                            continue;
                        }
                    }
                }
            }

            if let Some(e) = last_error {
                return Err(format!("Discord send failed after {} retries: {}", MAX_RETRIES, e));
            }

            // Small delay between chunks to avoid rate limiting
            if chunks.len() > 1 {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Channel for DiscordChannel {
    fn name(&self) -> &str {
        "discord"
    }

    async fn start(&mut self, bus: &MessageBus) -> ChannelResult<()> {
        info!("Starting Discord channel...");
        *self.running.lock().await = true;
        self.run(bus).await;
        Ok(())
    }

    async fn stop(&mut self) -> ChannelResult<()> {
        *self.running.lock().await = false;
        Ok(())
    }

    fn is_enabled(&self) -> bool {
        self.config.enabled && !self.config.token.is_empty()
    }
}

impl DiscordChannel {
    async fn run(&self, bus: &MessageBus) {
        let mut reconnect_delay = 1u64;

        // Spawn outbound message handler (sends replies back to Discord via REST API)
        let token_for_outbound = self.config.token.clone();
        let mut outbound_rx = bus.subscribe_outbound();
        let running_for_outbound = self.running.clone();

        tokio::spawn(async move {
            loop {
                if !*running_for_outbound.lock().await {
                    break;
                }
                match outbound_rx.recv().await {
                    Ok(msg) => {
                        if msg.channel != "discord" {
                            continue;
                        }
                        info!("Sending Discord reply to channel {}", msg.chat_id);
                        if let Err(e) = Self::send_message(&token_for_outbound, &msg.chat_id, &msg.content).await {
                            error!("Failed to send Discord reply: {}", e);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("Outbound receiver lagged by {} messages", n);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        });

        loop {
            if !*self.running.lock().await {
                break;
            }

            info!("Connecting to Discord Gateway...");

            let gateway_url = self.config.gateway_url.clone();
            info!("Gateway URL: {}", gateway_url);

            let (ws_stream, _) = match tokio_tungstenite::connect_async(&gateway_url).await {
                Ok(stream) => {
                    info!("WebSocket handshake successful");
                    reconnect_delay = 1; // Reset on successful connect
                    stream
                }
                Err(e) => {
                    error!("Failed to connect to Gateway: {}", e);
                    info!("Will retry in {} seconds...", reconnect_delay);
                    tokio::time::sleep(Duration::from_secs(reconnect_delay)).await;
                    reconnect_delay = (reconnect_delay * 2).min(60);
                    continue;
                }
            };

            let (ws_sender, mut ws_receiver) = ws_stream.split();

            // Use mpsc to share the ws_sender between heartbeat and identify/message handling
            let (ws_tx, mut ws_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

            // Task that forwards messages from mpsc to WebSocket
            let sender_task = tokio::spawn(async move {
                let mut ws_sender = ws_sender;
                while let Some(msg) = ws_rx.recv().await {
                    if let Err(e) = ws_sender.send(
                        tokio_tungstenite::tungstenite::protocol::Message::Text(msg)
                    ).await {
                        error!("WebSocket send failed: {}", e);
                        break;
                    }
                }
            });

            // Heartbeat task
            let ws_tx_heartbeat = ws_tx.clone();
            let running_clone = self.running.clone();
            let sequence_clone = self.sequence.clone();
            let heartbeat_interval_ms = Arc::new(Mutex::new(41250u64));
            let heartbeat_interval_clone = heartbeat_interval_ms.clone();

            let heartbeat_task = tokio::spawn(async move {
                // Wait a bit for Hello to set the interval
                tokio::time::sleep(Duration::from_secs(2)).await;

                loop {
                    if !*running_clone.lock().await {
                        break;
                    }

                    let interval_ms = *heartbeat_interval_clone.lock().await;
                    tokio::time::sleep(Duration::from_millis(interval_ms)).await;

                    if !*running_clone.lock().await {
                        break;
                    }

                    let seq = *sequence_clone.lock().await;
                    let heartbeat = json!({
                        "op": OpCode::Heartbeat as i64,
                        "d": seq
                    });

                    if ws_tx_heartbeat.send(heartbeat.to_string()).is_err() {
                        break;
                    }
                }
            });

            // Message receiving loop
            while let Some(result) = ws_receiver.next().await {
                if !*self.running.lock().await {
                    break;
                }

                match result {
                    Ok(msg) => {
                        if let Ok(text) = msg.to_text() {
                            if let Ok(gateway_msg) = serde_json::from_str::<GatewayMessage>(text) {
                                // Update sequence number
                                if let Some(s) = gateway_msg.s {
                                    *self.sequence.lock().await = Some(s);
                                }

                                self.handle_gateway_message(&gateway_msg, bus, &ws_tx, &heartbeat_interval_ms).await;
                            }
                        }
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                }
            }

            // Cleanup
            heartbeat_task.abort();
            sender_task.abort();

            if !*self.running.lock().await {
                break;
            }

            info!("Gateway disconnected, will reconnect in {} seconds...", reconnect_delay);
            tokio::time::sleep(Duration::from_secs(reconnect_delay)).await;
            reconnect_delay = (reconnect_delay * 2).min(60);
        }

        info!("Discord gateway connection closed");
    }

    async fn handle_gateway_message(
        &self,
        msg: &GatewayMessage,
        bus: &MessageBus,
        ws_tx: &tokio::sync::mpsc::UnboundedSender<String>,
        heartbeat_interval_ms: &Arc<Mutex<u64>>,
    ) {
        if let Some(op) = OpCode::from_i64(msg.op) {
            match op {
                OpCode::Hello => {
                    if let Some(d) = &msg.d {
                        if let Ok(payload) = serde_json::from_value::<HelloPayload>(d.clone()) {
                            *heartbeat_interval_ms.lock().await = payload.heartbeat_interval;
                            info!("Received Hello, heartbeat interval: {}ms", payload.heartbeat_interval);
                        }
                    }

                    // Send Identify
                    let identify = json!({
                        "op": OpCode::Identify as i64,
                        "d": {
                            "token": self.config.token,
                            "intents": self.config.intents,
                            "properties": {
                                "os": "linux",
                                "browser": "openat",
                                "device": "openat"
                            }
                        }
                    });
                    info!("Sending Identify...");
                    if ws_tx.send(identify.to_string()).is_err() {
                        error!("Failed to send Identify");
                    }
                }
                OpCode::HeartbeatAck => {
                    info!("Heartbeat acknowledged");
                }
                OpCode::Dispatch => {
                    let event_name = msg.t.as_ref().map(|s| s.as_str()).unwrap_or("");
                    match event_name {
                        "READY" => {
                            info!("Discord bot is READY!");
                            if let Some(d) = &msg.d {
                                if let Some(user) = d.get("user") {
                                    let username = user.get("username").and_then(|v| v.as_str()).unwrap_or("unknown");
                                    info!("Logged in as: {}", username);
                                }
                            }
                        }
                        "MESSAGE_CREATE" => {
                            if let Some(d) = &msg.d {
                                self.process_message(d, bus).await;
                            }
                        }
                        _ => {
                            info!("Received event: {}", event_name);
                        }
                    }
                }
                OpCode::InvalidSession => {
                    error!("Invalid session, will reconnect...");
                }
                OpCode::Reconnect => {
                    info!("Gateway requested reconnect");
                }
                _ => {}
            }
        }
    }

    async fn process_message(&self, message: &serde_json::Value, bus: &MessageBus) {
        // Skip bot messages
        if message.get("author").and_then(|a| a.get("bot")).and_then(|b| b.as_bool()).unwrap_or(false) {
            return;
        }

        let content = message.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();

        // Remove @mention prefix
        let mut content = content;
        if content.starts_with("<@") {
            if let Some(end) = content.find(">") {
                content = content[end + 1..].trim_start().to_string();
            }
        }

        if content.is_empty() {
            return;
        }

        let channel_id = message.get("channel_id").and_then(|v| v.as_str()).unwrap_or("");
        let sender_id = message.get("author").and_then(|a| a.get("id")).and_then(|v| v.as_str()).unwrap_or("");

        info!("Discord message from {} in {}: {}", sender_id, channel_id, content);

        if !self.is_allowed(sender_id) {
            info!("User {} not in allowed list, ignoring", sender_id);
            return;
        }

        let inbound = InboundMessage::new("discord", sender_id, channel_id, &content);
        bus.publish_inbound(inbound).await;
    }
}

/// Split a message into chunks that fit within Discord's character limit.
/// Tries to split on newlines first, then on spaces, then hard-splits.
fn split_message(content: &str, max_len: usize) -> Vec<String> {
    if content.len() <= max_len {
        return vec![content.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = content;

    while !remaining.is_empty() {
        if remaining.len() <= max_len {
            chunks.push(remaining.to_string());
            break;
        }

        // Find a char-boundary-safe upper bound
        let mut upper = max_len;
        while upper > 0 && !remaining.is_char_boundary(upper) {
            upper -= 1;
        }

        // Try to find a good split point (newline, then space)
        let split_at = remaining[..upper]
            .rfind('\n')
            .or_else(|| remaining[..upper].rfind(' '))
            .unwrap_or(upper);

        let (chunk, rest) = remaining.split_at(split_at);
        chunks.push(chunk.to_string());
        remaining = rest.trim_start_matches('\n').trim_start_matches(' ');
    }

    chunks
}
