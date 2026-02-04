use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::broadcast;

/// Message received from a chat channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    pub channel: String,
    pub sender_id: String,
    pub chat_id: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub media: Vec<String>,
    pub metadata: HashMap<String, String>,
}

impl InboundMessage {
    pub fn new(
        channel: impl Into<String>,
        sender_id: impl Into<String>,
        chat_id: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            channel: channel.into(),
            sender_id: sender_id.into(),
            chat_id: chat_id.into(),
            content: content.into(),
            timestamp: Utc::now(),
            media: vec![],
            metadata: HashMap::new(),
        }
    }

    pub fn session_key(&self) -> String {
        format!("{}:{}", self.channel, self.chat_id)
    }
}

/// Message to send to a chat channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundMessage {
    pub channel: String,
    pub chat_id: String,
    pub content: String,
    pub reply_to: Option<String>,
    pub media: Vec<String>,
    pub metadata: HashMap<String, String>,
}

impl OutboundMessage {
    pub fn new(
        channel: impl Into<String>,
        chat_id: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            channel: channel.into(),
            chat_id: chat_id.into(),
            content: content.into(),
            reply_to: None,
            media: vec![],
            metadata: HashMap::new(),
        }
    }
}

/// Event types for message bus
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    #[serde(rename = "message")]
    Message(InboundMessage),
    #[serde(rename = "connect")]
    Connect { channel: String, chat_id: String },
    #[serde(rename = "disconnect")]
    Disconnect { channel: String, chat_id: String },
    #[serde(rename = "error")]
    Error { channel: String, error: String },
}

/// Async message bus for decoupled channel-agent communication
#[derive(Debug, Clone)]
pub struct MessageBus {
    inbound_tx: broadcast::Sender<InboundMessage>,
    outbound_tx: broadcast::Sender<OutboundMessage>,
}

impl MessageBus {
    pub fn new() -> Self {
        let (inbound_tx, _) = broadcast::channel(100);
        let (outbound_tx, _) = broadcast::channel(100);

        Self {
            inbound_tx,
            outbound_tx,
        }
    }

    /// Publish an inbound message
    pub async fn publish_inbound(&self, msg: InboundMessage) {
        let _ = self.inbound_tx.send(msg);
    }

    /// Subscribe to inbound messages
    pub fn subscribe_inbound(&self) -> broadcast::Receiver<InboundMessage> {
        self.inbound_tx.subscribe()
    }

    /// Publish an outbound message
    pub async fn publish_outbound(&self, msg: OutboundMessage) {
        let _ = self.outbound_tx.send(msg);
    }

    /// Subscribe to outbound messages
    pub fn subscribe_outbound(&self) -> broadcast::Receiver<OutboundMessage> {
        self.outbound_tx.subscribe()
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}
