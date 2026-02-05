use crate::bus::{InboundMessage, MessageBus};
use crate::config::Config;
use futures_util::stream::StreamExt;
use futures_util::SinkExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use tracing::{debug, error, info, warn};

/// OneBot v11 event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "post_type")]
enum OneBotEvent {
    #[serde(rename = "message")]
    Message(MessageEvent),
    #[serde(rename = "notice")]
    Notice(NoticeEvent),
    #[serde(rename = "request")]
    Request(RequestEvent),
    #[serde(rename = "meta_event")]
    Meta(MetaEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MessageEvent {
    #[serde(rename = "message_type")]
    message_type: String,
    #[serde(rename = "message_id")]
    message_id: i64,
    #[serde(rename = "user_id")]
    user_id: Option<i64>,
    #[serde(rename = "group_id")]
    group_id: Option<i64>,
    #[serde(rename = "message")]
    msg_content: String,
    #[serde(rename = "sender")]
    sender: Option<SenderInfo>,
    #[serde(rename = "time")]
    time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NoticeEvent {
    #[serde(rename = "notice_type")]
    notice_type: String,
    #[serde(rename = "user_id")]
    user_id: Option<i64>,
    #[serde(rename = "group_id")]
    group_id: Option<i64>,
    #[serde(rename = "time")]
    time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RequestEvent {
    #[serde(rename = "request_type")]
    request_type: String,
    #[serde(rename = "user_id")]
    user_id: Option<i64>,
    #[serde(rename = "group_id")]
    group_id: Option<i64>,
    #[serde(rename = "time")]
    time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetaEvent {
    #[serde(rename = "meta_event_type")]
    meta_event_type: String,
    #[serde(rename = "time")]
    time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SenderInfo {
    #[serde(rename = "user_id")]
    user_id: i64,
    #[serde(rename = "nickname")]
    nickname: Option<String>,
    #[serde(rename = "card")]
    card: Option<String>,
}

/// QQ channel implementation using OneBot v11 protocol
pub struct QQChannel {
    /// HTTP API URL
    api_url: String,
    /// WebSocket event URL
    event_url: String,
    /// Access token for authentication
    access_token: Option<String>,
    /// Message bus for publishing inbound messages
    bus: MessageBus,
    /// Allowed user IDs
    allowed_users: Vec<String>,
    /// Shutdown flag
    running: Arc<AtomicBool>,
}

impl QQChannel {
    /// Create a new QQ channel
    pub fn new(api_url: String, event_url: String, access_token: Option<String>, bus: MessageBus, allowed_users: Vec<String>) -> Self {
        Self {
            api_url,
            event_url,
            access_token,
            bus,
            allowed_users,
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    fn is_allowed(&self, user_id: &str) -> bool {
        self.allowed_users.is_empty() || self.allowed_users.contains(&user_id.to_string())
    }

    /// Start the QQ channel event loop
    pub async fn run(&mut self) -> anyhow::Result<()> {
        info!("Connecting to OneBot WebSocket at {}...", self.event_url);

        let (ws_stream, _) = connect_async(&self.event_url).await
            .map_err(|e| anyhow::anyhow!("Failed to connect to OneBot WebSocket: {}", e))?;

        info!("Connected to OneBot WebSocket");

        let (mut write, mut read) = ws_stream.split();

        let bus = self.bus.clone();
        let allowed_users = self.allowed_users.clone();
        let running = self.running.clone();

        // Handle incoming messages
        tokio::spawn(async move {
            while running.load(Ordering::Relaxed) {
                match read.next().await {
                    Some(Ok(WsMessage::Text(text))) => {
                        if let Ok(event) = serde_json::from_str::<OneBotEvent>(&text) {
                            if let OneBotEvent::Message(msg_event) = event {
                                let user_id = msg_event.user_id.map(|id| id.to_string()).unwrap_or_default();
                                let group_id = msg_event.group_id.map(|id| id.to_string());
                                let content = msg_event.msg_content.clone();

                                if content.is_empty() {
                                    continue;
                                }

                                // Check if user is allowed
                                if !allowed_users.is_empty() && !allowed_users.contains(&user_id) {
                                    warn!("User {} is not authorized", user_id);
                                    continue;
                                }

                                debug!("Received QQ message from user {}: {}", user_id, content);

                                let chat_id = group_id.clone().unwrap_or_else(|| user_id.clone());

                                // Create and publish inbound message
                                let inbound = InboundMessage::new(
                                    "qq",
                                    &user_id,
                                    &chat_id,
                                    &content,
                                );

                                bus.publish_inbound(inbound).await;
                            }
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) => {
                        warn!("OneBot WebSocket connection closed");
                        break;
                    }
                    Some(Ok(WsMessage::Ping(_))) | Some(Ok(WsMessage::Pong(_))) => {
                        // Ignore heartbeat messages
                    }
                    Some(Ok(WsMessage::Binary(_))) => {
                        // Ignore binary messages
                    }
                    Some(Ok(WsMessage::Frame(_))) => {
                        // Ignore frame messages
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        break;
                    }
                }
            }
        });

        // Send heartbeat every 30 seconds
        let running_heartbeat = self.running.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            while running_heartbeat.load(Ordering::Relaxed) {
                interval.tick().await;
                if let Err(e) = write.send(WsMessage::Text(json!({
                    "action": "send_packets",
                    "params": {},
                    "echo": "heartbeat"
                }).to_string())).await {
                    error!("Failed to send heartbeat: {}", e);
                    break;
                }
            }
        });

        Ok(())
    }

    /// Stop the channel
    pub async fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        info!("QQ channel stopped");
    }

    /// Send a private message via OneBot HTTP API
    pub async fn send_private_msg(&self, user_id: i64, content: &str) -> anyhow::Result<()> {
        self.call_api("send_private_msg", json!({
            "user_id": user_id,
            "message": content,
        })).await
    }

    /// Send a group message via OneBot HTTP API
    pub async fn send_group_msg(&self, group_id: i64, content: &str) -> anyhow::Result<()> {
        self.call_api("send_group_msg", json!({
            "group_id": group_id,
            "message": content,
        })).await
    }

    /// Call OneBot HTTP API
    async fn call_api(&self, action: &str, params: serde_json::Value) -> anyhow::Result<()> {
        let url = format!("{}/api/{}", self.api_url, action);

        let mut request = reqwest::Client::new()
            .post(&url)
            .json(&json!({
                "action": action,
                "params": params,
            }));

        if let Some(token) = &self.access_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await
            .map_err(|e| anyhow::anyhow!("API request failed: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("API error: {}", error_text));
        }

        Ok(())
    }
}

/// Start the QQ channel with OneBot connection
pub async fn start_qq_channel(config: &Config, bus: &MessageBus) -> anyhow::Result<QQChannel> {
    let qq_config = &config.channels.qq;

    if !qq_config.enabled {
        warn!("QQ channel is not enabled in config");
        return Err(anyhow::anyhow!("QQ channel is not enabled"));
    }

    if qq_config.event_url.is_empty() {
        warn!("QQ event URL not configured");
        return Err(anyhow::anyhow!("QQ event URL not configured"));
    }

    let access_token = if qq_config.access_token.is_empty() {
        None
    } else {
        Some(qq_config.access_token.clone())
    };

    let allowed_users: Vec<String> = qq_config.allowed_users
        .iter()
        .cloned()
        .collect();

    let mut channel = QQChannel::new(
        qq_config.api_url.clone(),
        qq_config.event_url.clone(),
        access_token,
        bus.clone(),
        allowed_users,
    );

    channel.run().await?;

    Ok(channel)
}
