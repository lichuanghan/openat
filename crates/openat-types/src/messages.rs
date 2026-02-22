//! Message types

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

/// Role of the message sender
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageRole {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "system")]
    System,
    #[serde(rename = "tool")]
    Tool,
}

/// Message content with optional media
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageContent {
    pub text: Option<String>,
    pub media: Vec<MediaItem>,
}

impl MessageContent {
    pub fn new(text: Option<String>) -> Self {
        Self { text, media: vec![] }
    }

    pub fn with_media(text: Option<String>, media: Vec<MediaItem>) -> Self {
        Self { text, media }
    }
}

/// Media attachment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaItem {
    pub r#type: String,      // image, audio, video, file
    pub url: String,
    pub name: Option<String>,
    pub size: Option<usize>,
}

/// Inbound message from a channel
#[derive(Debug, Clone)]
pub struct InboundMessage {
    pub channel: String,
    pub sender_id: String,
    pub chat_id: String,
    pub content: String,
    pub timestamp: chrono::DateTime<Utc>,
    pub media: Vec<MediaItem>,
    pub metadata: HashMap<String, String>,
}

impl InboundMessage {
    pub fn new(channel: &str, sender_id: &str, chat_id: &str, content: &str) -> Self {
        Self {
            channel: channel.to_string(),
            sender_id: sender_id.to_string(),
            chat_id: chat_id.to_string(),
            content: content.to_string(),
            timestamp: Utc::now(),
            media: vec![],
            metadata: HashMap::new(),
        }
    }

    pub fn with_media(mut self, media: Vec<MediaItem>) -> Self {
        self.media = media;
        self
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    pub fn session_key(&self) -> String {
        format!("{}:{}", self.channel, self.chat_id)
    }
}

/// Outbound message to a channel
#[derive(Debug, Clone)]
pub struct OutboundMessage {
    pub channel: String,
    pub chat_id: String,
    pub content: String,
    pub reply_to: Option<String>,
}

impl OutboundMessage {
    pub fn new(channel: &str, chat_id: &str, content: &str) -> Self {
        Self {
            channel: channel.to_string(),
            chat_id: chat_id.to_string(),
            content: content.to_string(),
            reply_to: None,
        }
    }

    pub fn reply(mut self, message_id: &str) -> Self {
        self.reply_to = Some(message_id.to_string());
        self
    }
}

/// Convert to LLM message format
impl From<&InboundMessage> for serde_json::Value {
    fn from(msg: &InboundMessage) -> Self {
        json!({
            "role": "user",
            "content": msg.content
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inbound_message_new() {
        let msg = InboundMessage::new("discord", "user123", "channel456", "Hello");
        assert_eq!(msg.channel, "discord");
        assert_eq!(msg.sender_id, "user123");
        assert_eq!(msg.chat_id, "channel456");
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_inbound_session_key() {
        let msg = InboundMessage::new("discord", "user123", "channel456", "Hello");
        assert_eq!(msg.session_key(), "discord:channel456");
    }

    #[test]
    fn test_outbound_message_reply() {
        let msg = OutboundMessage::new("discord", "channel456", "Hello")
            .reply("msg123");
        assert_eq!(msg.reply_to, Some("msg123".to_string()));
    }
}
