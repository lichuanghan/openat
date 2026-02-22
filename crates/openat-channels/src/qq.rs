//! QQ Official Bot API channel adapter
//!
//! Connects to QQ Open Platform via WebSocket gateway,
//! receives messages and sends replies via REST API.

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Duration;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use tracing::{debug, error, info, warn};
use async_trait::async_trait;

use openat_runtime::MessageBus;
use openat_types::InboundMessage;
use crate::Channel;

type ChannelResult<T = ()> = anyhow::Result<T>;

use openat_config::QQ as QQConfig;

const TOKEN_URL: &str = "https://bots.qq.com/app/getAppAccessToken";
const API_BASE: &str = "https://api.sgroup.qq.com";
const SANDBOX_API_BASE: &str = "https://sandbox.api.sgroup.qq.com";

/// QQ Gateway opcodes (same as Discord)
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

#[derive(Debug, Deserialize)]
struct HelloPayload {
    heartbeat_interval: u64,
}

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

#[derive(Debug, Deserialize)]
struct GatewayResponse {
    url: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    expires_in: Option<String>,
}

/// QQ channel implementation
#[derive(Clone)]
pub struct QQChannel {
    config: QQConfig,
    http_client: Arc<reqwest::Client>,
    running: Arc<Mutex<bool>>,
    sequence: Arc<Mutex<Option<u64>>>,
    access_token: Arc<Mutex<String>>,
    session_id: Arc<Mutex<Option<String>>>,
}

impl QQChannel {
    pub fn new(config: QQConfig) -> Self {
        Self {
            config,
            http_client: Arc::new(reqwest::Client::new()),
            running: Arc::new(Mutex::new(false)),
            sequence: Arc::new(Mutex::new(None)),
            access_token: Arc::new(Mutex::new(String::new())),
            session_id: Arc::new(Mutex::new(None)),
        }
    }

    fn api_base(&self) -> &str {
        if self.config.sandbox {
            SANDBOX_API_BASE
        } else {
            API_BASE
        }
    }

    fn is_allowed(&self, user_id: &str) -> bool {
        self.config.allowed_users.is_empty()
            || self.config.allowed_users.iter().any(|u| u == user_id)
    }

    /// Obtain or refresh the access token
    async fn refresh_token(&self) -> Result<String, String> {
        let resp = self.http_client
            .post(TOKEN_URL)
            .json(&json!({
                "appId": self.config.app_id,
                "clientSecret": self.config.client_secret,
            }))
            .send()
            .await
            .map_err(|e| format!("Token request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Token API error {}: {}", status, body));
        }

        let token_resp: TokenResponse = resp
            .json()
            .await
            .map_err(|e| format!("Token parse error: {}", e))?;

        info!("Got access token, expires in: {:?}", token_resp.expires_in);
        *self.access_token.lock().await = token_resp.access_token.clone();
        Ok(token_resp.access_token)
    }

    /// Get the WebSocket gateway URL
    async fn get_gateway_url(&self) -> Result<String, String> {
        let token = self.access_token.lock().await.clone();
        let url = format!("{}/gateway/bot", self.api_base());
        debug!("Fetching QQ gateway URL from: {}", url);

        let resp = self.http_client
            .get(&url)
            .header("Authorization", format!("QQBot {}", token))
            .send()
            .await
            .map_err(|e| format!("Gateway URL request failed: {}", e))?;

        debug!("Gateway URL response status: {}", resp.status());

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Gateway API error {}: {}", status, body));
        }

        let gateway: GatewayResponse = resp
            .json()
            .await
            .map_err(|e| format!("Gateway parse error: {}", e))?;

        debug!("QQ gateway URL: {}", gateway.url);
        Ok(gateway.url)
    }

    /// Send a message via REST API
    async fn send_message(
        client: &Arc<reqwest::Client>,
        api_base: &str,
        token: &str,
        msg_type: &str,
        target_id: &str,
        content: &str,
        msg_id: Option<&str>,
    ) -> Result<(), String> {
        // QQ limit is 2000 chars per message
        let chunks = split_message(content, 2000);

        for (i, chunk) in chunks.iter().enumerate() {
            let url = match msg_type {
                "group" => format!("{}/v2/groups/{}/messages", api_base, target_id),
                "c2c" => format!("{}/v2/users/{}/messages", api_base, target_id),
                _ => return Err(format!("Unknown message type: {}", msg_type)),
            };

            let mut body = json!({
                "content": chunk,
                "msg_type": 0,
            });

            // Only include msg_id for the first chunk when sending multiple chunks
            if i == 0 {
                if let Some(id) = msg_id {
                    body["msg_id"] = json!(id);
                }
            }

            let resp = client
                .post(&url)
                .header("Authorization", format!("QQBot {}", token))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("Failed to send QQ message: {}", e))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                // If URL blocked, try stripping URLs and retry once
                if body.contains("40034028") && i == 0 {
                    let stripped = strip_urls(chunk);
                    if stripped != *chunk {
                        let retry_body = json!({
                            "content": stripped,
                            "msg_type": if i == 0 && msg_id.is_some() { 0 } else { 0 },
                        });
                        let resp2 = client
                            .post(&url)
                            .header("Authorization", format!("QQBot {}", token))
                            .header("Content-Type", "application/json")
                            .json(&retry_body)
                            .send()
                            .await
                            .map_err(|e| format!("Failed to send QQ message: {}", e))?;

                        if !resp2.status().is_success() {
                            let status = resp2.status();
                            let body = resp2.text().await.unwrap_or_default();
                            return Err(format!("QQ API error {}: {}", status, body));
                        }
                        continue;
                    }
                }
                return Err(format!("QQ API error {}: {}", status, body));
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
impl Channel for QQChannel {
    fn name(&self) -> &str {
        "qq"
    }

    async fn start(&mut self, bus: &MessageBus) -> ChannelResult<()> {
        info!("Starting QQ channel...");
        *self.running.lock().await = true;
        self.run(bus).await;
        Ok(())
    }

    async fn stop(&mut self) -> ChannelResult<()> {
        *self.running.lock().await = false;
        Ok(())
    }

    fn is_enabled(&self) -> bool {
        self.config.enabled && !self.config.app_id.is_empty() && !self.config.client_secret.is_empty()
    }
}

impl QQChannel {
    async fn run(&self, bus: &MessageBus) {
        let mut reconnect_delay = 1u64;

        // Obtain initial access token
        match self.refresh_token().await {
            Ok(_) => info!("QQ access token obtained"),
            Err(e) => {
                error!("Failed to get QQ access token: {}", e);
                return;
            }
        }

        // Spawn token refresh task (refresh every 100 minutes, token expires in 120 min)
        {
            let this = self.clone();
            tokio::spawn(async move {
                info!("QQ token refresh task started");
                loop {
                    tokio::time::sleep(Duration::from_secs(6000)).await;
                    if !*this.running.lock().await {
                        info!("QQ token refresh task stopping");
                        break;
                    }
                    info!("QQ token refresh - about to call refresh_token()");
                    match this.refresh_token().await {
                        Ok(_) => info!("QQ access token refreshed"),
                        Err(e) => error!("Failed to refresh QQ token: {}", e),
                    }
                }
            });
        }

        // Spawn outbound message handler with queue and retry
        {
            use std::collections::VecDeque;
            use std::time::Instant;

            let access_token = self.access_token.clone();
            let http_client = self.http_client.clone();
            let api_base = self.api_base().to_string();
            let mut outbound_rx = bus.subscribe_outbound();
            let running = self.running.clone();

            // Queue: (message, retry_count, next_retry_time)
            let queue: Arc<Mutex<VecDeque<(openat_types::OutboundMessage, u8, Option<Instant>)>>> =
                Arc::new(Mutex::new(VecDeque::new()));
            let queue_for_sender = queue.clone();

            // Task that processes the queue
            let queue_for_processor = queue.clone();
            let running_for_processor = running.clone();
            tokio::spawn(async move {
                const MAX_RETRIES: u8 = 3;
                const BASE_DELAY_MS: u64 = 1000;

                loop {
                    if !*running_for_processor.lock().await {
                        break;
                    }

                    // Process queue
                    let should_sleep = {
                        let mut q = queue_for_processor.lock().await;
                        let now = Instant::now();

                        // Find messages ready to send
                        let mut i = 0;
                        while i < q.len() {
                            let (_, _, next_retry) = &q[i];
                            if next_retry.map_or(true, |t| t <= now) {
                                break;
                            }
                            i += 1;
                        }

                        // Process ready messages
                        while let Some((msg, retry_count, _)) = q.pop_front() {
                            let token = access_token.lock().await.clone();
                            let (msg_type, target_id) = if msg.chat_id.starts_with("group:") {
                                ("group", &msg.chat_id[6..])
                            } else if msg.chat_id.starts_with("c2c:") {
                                ("c2c", &msg.chat_id[4..])
                            } else {
                                ("group", msg.chat_id.as_str())
                            };
                            let msg_id = msg.reply_to.as_deref();

                            let result = QQChannel::send_message(
                                &http_client, &api_base, &token, msg_type, target_id, &msg.content, msg_id,
                            ).await;

                            match result {
                                Ok(_) => {
                                    info!("QQ reply sent successfully");
                                }
                                Err(e) => {
                                    let current_retry = retry_count;
                                    if current_retry < MAX_RETRIES {
                                        let delay = BASE_DELAY_MS * (2u64.pow(current_retry as u32)).min(30000);
                                        let next_retry = Instant::now() + Duration::from_millis(delay);
                                        q.push_back((msg, current_retry + 1, Some(next_retry)));
                                        warn!("QQ reply failed, retry {}/{} after {}ms: {}",
                                            current_retry + 1, MAX_RETRIES, delay, e);
                                    } else {
                                        error!("QQ reply failed after {} retries: {}", MAX_RETRIES, e);
                                    }
                                }
                            }
                        }

                        // Check if queue is empty
                        q.is_empty()
                    };

                    // Sleep if queue is empty or wait a bit before checking again
                    if should_sleep {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    } else {
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                }
            });

            // Task that receives messages and adds to queue
            let running_for_receiver = running.clone();
            tokio::spawn(async move {
                loop {
                    if !*running_for_receiver.lock().await {
                        break;
                    }
                    match outbound_rx.recv().await {
                        Ok(msg) => {
                            if msg.channel != "qq" {
                                continue;
                            }
                            // Add to queue with 0 retries (clone msg since it's Arc)
                            queue_for_sender.lock().await.push_back(((*msg).clone(), 0, None));
                            debug!("QQ message added to outbound queue");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            warn!("QQ outbound receiver lagged by {} messages", n);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            break;
                        }
                    }
                }
            });
        }

        // Main gateway connection loop
        loop {
            if !*self.running.lock().await {
                break;
            }

            // Get gateway URL
            let gateway_url = match self.get_gateway_url().await {
                Ok(url) => url,
                Err(e) => {
                    error!("Failed to get QQ gateway URL: {}", e);
                    tokio::time::sleep(Duration::from_secs(reconnect_delay)).await;
                    reconnect_delay = (reconnect_delay * 2).min(60);
                    continue;
                }
            };

            info!("Connecting to QQ Gateway: {}", gateway_url);

            let (ws_stream, _) = match tokio_tungstenite::connect_async(&gateway_url).await {
                Ok(stream) => {
                    info!("QQ WebSocket handshake successful");
                    reconnect_delay = 1;
                    stream
                }
                Err(e) => {
                    error!("Failed to connect to QQ Gateway: {}", e);
                    tokio::time::sleep(Duration::from_secs(reconnect_delay)).await;
                    reconnect_delay = (reconnect_delay * 2).min(60);
                    continue;
                }
            };

            let (ws_sender, mut ws_receiver) = ws_stream.split();

            // mpsc channel for sharing ws_sender
            let (ws_tx, mut ws_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

            // WebSocket sender task
            let sender_task = tokio::spawn(async move {
                let mut ws_sender = ws_sender;
                while let Some(msg) = ws_rx.recv().await {
                    if let Err(e) = ws_sender.send(
                        tokio_tungstenite::tungstenite::protocol::Message::Text(msg)
                    ).await {
                        error!("QQ WebSocket send failed: {}", e);
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
                                if let Some(s) = gateway_msg.s {
                                    *self.sequence.lock().await = Some(s);
                                }
                                self.handle_gateway_message(&gateway_msg, bus, &ws_tx, &heartbeat_interval_ms).await;
                            }
                        }
                    }
                    Err(e) => {
                        error!("QQ WebSocket error: {}", e);
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

            info!("QQ Gateway disconnected, will reconnect in {} seconds...", reconnect_delay);
            tokio::time::sleep(Duration::from_secs(reconnect_delay)).await;
            reconnect_delay = (reconnect_delay * 2).min(60);
        }

        info!("QQ gateway connection closed");
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
                            info!("QQ Hello received, heartbeat interval: {}ms", payload.heartbeat_interval);
                        }
                    }

                    // Send Identify
                    let token = self.access_token.lock().await.clone();
                    // Compute intents from boolean config
                    // GROUP_AT_MESSAGE_CREATE = 1 << 26 = 67108864
                    // C2C_MESSAGE_CREATE = 1 << 25 = 33554432
                    // AT_MESSAGE_CREATE (guild) = 1 << 30
                    let mut intents: u32 = 0;
                    if self.config.listen_group {
                        intents |= 1 << 26; // GROUP_AT_MESSAGE_CREATE
                    }
                    if self.config.listen_private {
                        intents |= 1 << 25; // C2C_MESSAGE_CREATE
                    }
                    if self.config.listen_guild {
                        intents |= 1 << 30; // AT_MESSAGE_CREATE
                    }
                    if intents == 0 {
                        // Default: enable all
                        intents = (1 << 26) | (1 << 25) | (1 << 30);
                    }
                    info!("QQ intents computed: {} (group:{}, private:{}, guild:{})",
                        intents, self.config.listen_group, self.config.listen_private, self.config.listen_guild);

                    let identify = json!({
                        "op": OpCode::Identify as i64,
                        "d": {
                            "token": format!("QQBot {}", token),
                            "intents": intents,
                            "shard": [0, 1],
                        }
                    });
                    info!("Sending QQ Identify (intents: {})...", intents);
                    if ws_tx.send(identify.to_string()).is_err() {
                        error!("Failed to send QQ Identify");
                    }
                }
                OpCode::HeartbeatAck => {
                    info!("QQ Heartbeat acknowledged");
                }
                OpCode::Dispatch => {
                    let event_name = msg.t.as_ref().map(|s| s.as_str()).unwrap_or("");
                    match event_name {
                        "READY" => {
                            info!("QQ bot is READY!");
                            if let Some(d) = &msg.d {
                                if let Some(session_id) = d.get("session_id").and_then(|v| v.as_str()) {
                                    *self.session_id.lock().await = Some(session_id.to_string());
                                    info!("QQ session_id: {}", session_id);
                                }
                                if let Some(user) = d.get("user") {
                                    let username = user.get("username").and_then(|v| v.as_str()).unwrap_or("unknown");
                                    info!("QQ logged in as: {}", username);
                                }
                            }
                        }
                        "GROUP_AT_MESSAGE_CREATE" => {
                            if let Some(d) = &msg.d {
                                self.process_group_message(d, bus).await;
                            }
                        }
                        "C2C_MESSAGE_CREATE" => {
                            if let Some(d) = &msg.d {
                                self.process_c2c_message(d, bus).await;
                            }
                        }
                        "AT_MESSAGE_CREATE" => {
                            // Guild text channel @bot message
                            if let Some(d) = &msg.d {
                                self.process_guild_message(d, bus).await;
                            }
                        }
                        _ => {
                            info!("QQ event: {}", event_name);
                        }
                    }
                }
                OpCode::InvalidSession => {
                    error!("QQ invalid session, will reconnect...");
                }
                OpCode::Reconnect => {
                    info!("QQ gateway requested reconnect");
                }
                _ => {}
            }
        }
    }

    /// Process group @bot message
    async fn process_group_message(&self, message: &serde_json::Value, bus: &MessageBus) {
        let content = message.get("content").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        if content.is_empty() {
            return;
        }

        let group_openid = message.get("group_openid").and_then(|v| v.as_str()).unwrap_or("");
        let author = message.get("author");
        let member_openid = author.and_then(|a| a.get("member_openid")).and_then(|v| v.as_str()).unwrap_or("");
        let msg_id = message.get("id").and_then(|v| v.as_str()).unwrap_or("");

        info!("QQ group message from {} in group {}: {}", member_openid, group_openid, content);

        if !self.is_allowed(member_openid) && !self.config.allowed_users.is_empty() {
            info!("QQ user {} not in allowed list, ignoring", member_openid);
            return;
        }

        // chat_id format: "group:{group_openid}" so outbound knows how to reply
        let chat_id = format!("group:{}", group_openid);
        let inbound = InboundMessage::new("qq", member_openid, &chat_id, &content)
            .with_metadata("msg_id", msg_id);
        bus.publish_inbound(inbound).await;
    }

    /// Process C2C (single chat) message
    async fn process_c2c_message(&self, message: &serde_json::Value, bus: &MessageBus) {
        let content = message.get("content").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        if content.is_empty() {
            return;
        }

        let author = message.get("author");
        let user_openid = author.and_then(|a| a.get("user_openid")).and_then(|v| v.as_str()).unwrap_or("");
        let msg_id = message.get("id").and_then(|v| v.as_str()).unwrap_or("");

        info!("QQ C2C message from {}: {}", user_openid, content);

        if !self.is_allowed(user_openid) && !self.config.allowed_users.is_empty() {
            info!("QQ user {} not in allowed list, ignoring", user_openid);
            return;
        }

        let chat_id = format!("c2c:{}", user_openid);
        let inbound = InboundMessage::new("qq", user_openid, &chat_id, &content)
            .with_metadata("msg_id", msg_id);
        bus.publish_inbound(inbound).await;
    }

    /// Process guild text channel @bot message
    async fn process_guild_message(&self, message: &serde_json::Value, bus: &MessageBus) {
        let content = message.get("content").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        if content.is_empty() {
            return;
        }

        let channel_id = message.get("channel_id").and_then(|v| v.as_str()).unwrap_or("");
        let author = message.get("author");
        let user_id = author.and_then(|a| a.get("id")).and_then(|v| v.as_str()).unwrap_or("");
        let msg_id = message.get("id").and_then(|v| v.as_str()).unwrap_or("");

        info!("QQ guild message from {} in channel {}: {}", user_id, channel_id, content);

        let chat_id = format!("guild:{}", channel_id);
        let inbound = InboundMessage::new("qq", user_id, &chat_id, &content)
            .with_metadata("msg_id", msg_id);
        bus.publish_inbound(inbound).await;
    }
}

/// Split a message into chunks that fit within QQ's character limit.
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

/// Strip URLs from content to bypass QQ's security filter.
/// Returns content with URLs removed.
fn strip_urls(content: &str) -> String {
    // Simple URL pattern: http://, https://, www.
    let mut result = content.to_string();

    // Remove URLs starting with http:// or https://
    while let Some(start) = result.find("http") {
        if let Some(end) = result[start..].find(|c: char| c.is_whitespace() || c == '>' || c == ')' || c == ']') {
            result = format!("{}{}", &result[..start], &result[start + end..]);
        } else {
            result = result[..start].to_string();
            break;
        }
    }

    // Remove www. URLs
    while let Some(start) = result.find("www.") {
        if let Some(end) = result[start..].find(|c: char| c.is_whitespace() || c == '>' || c == ')' || c == ']') {
            result = format!("{}{}", &result[..start], &result[start + end..]);
        } else {
            result = result[..start].to_string();
            break;
        }
    }

    result.trim().to_string()
}
