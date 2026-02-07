//! Shared types for the openat project.
//! This module contains types that are shared across multiple modules
//! to avoid circular dependencies.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Role of a message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "tool")]
    Tool,
}

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub name: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub tool_call_id: Option<String>,
}

impl Message {
    /// Create a system message
    pub fn system(content: &str) -> Self {
        Self {
            role: MessageRole::System,
            content: content.to_string(),
            name: None,
            tool_calls: vec![],
            tool_call_id: None,
        }
    }

    /// Create a user message
    pub fn user(content: &str) -> Self {
        Self {
            role: MessageRole::User,
            content: content.to_string(),
            name: None,
            tool_calls: vec![],
            tool_call_id: None,
        }
    }

    /// Create an assistant message
    pub fn assistant(content: &str) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.to_string(),
            name: None,
            tool_calls: vec![],
            tool_call_id: None,
        }
    }

    /// Create a tool result message
    pub fn tool(content: &str, tool_call_id: &str, name: &str) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.to_string(),
            name: Some(name.to_string()),
            tool_calls: vec![],
            tool_call_id: Some(tool_call_id.to_string()),
        }
    }

    /// Convert to JSON value for LLM API
    pub fn to_json(&self) -> Value {
        let mut map = serde_json::Map::new();
        let role_str = match self.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        };
        map.insert("role".to_string(), json!(role_str));

        if !self.content.is_empty() {
            map.insert("content".to_string(), serde_json::json!(self.content));
        }

        if let Some(name) = &self.name {
            map.insert("name".to_string(), serde_json::json!(name));
        }

        if let Some(tool_call_id) = &self.tool_call_id {
            map.insert("tool_call_id".to_string(), serde_json::json!(tool_call_id));
        }

        if !self.tool_calls.is_empty() {
            let tool_calls_json: Vec<Value> = self.tool_calls.iter().map(|tc| {
                serde_json::json!({
                    "id": tc.id,
                    "type": "function",
                    "function": {
                        "name": tc.name,
                        "arguments": tc.arguments
                    }
                })
            }).collect();
            map.insert("tool_calls".to_string(), serde_json::json!(tool_calls_json));
        }

        Value::Object(map)
    }
}

/// A tool call from the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

impl ToolCall {
    /// Create a new tool call
    pub fn new(id: &str, name: &str, arguments: Value) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            arguments,
        }
    }
}

/// Response from the LLM
#[derive(Debug, Clone)]
pub struct LLMResponse {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub finish_reason: String,
}

impl LLMResponse {
    /// Create a new LLM response
    pub fn new(content: Option<String>, tool_calls: Vec<ToolCall>, finish_reason: &str) -> Self {
        Self {
            content,
            tool_calls,
            finish_reason: finish_reason.to_string(),
        }
    }

    /// Create an empty response
    pub fn empty() -> Self {
        Self {
            content: Some(String::new()),
            tool_calls: vec![],
            finish_reason: "stop".to_string(),
        }
    }
}

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
    /// Create a new inbound message
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

    /// Generate a unique session key
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
    /// Create a new outbound message
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

    /// Create a reply to a message
    pub fn reply(
        channel: impl Into<String>,
        chat_id: impl Into<String>,
        content: impl Into<String>,
        reply_to: impl Into<String>,
    ) -> Self {
        Self {
            channel: channel.into(),
            chat_id: chat_id.into(),
            content: content.into(),
            reply_to: Some(reply_to.into()),
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

impl Event {
    /// Create a connection event
    pub fn connect(channel: &str, chat_id: &str) -> Self {
        Self::Connect {
            channel: channel.to_string(),
            chat_id: chat_id.to_string(),
        }
    }

    /// Create a disconnection event
    pub fn disconnect(channel: &str, chat_id: &str) -> Self {
        Self::Disconnect {
            channel: channel.to_string(),
            chat_id: chat_id.to_string(),
        }
    }

    /// Create an error event
    pub fn error(channel: &str, error: &str) -> Self {
        Self::Error {
            channel: channel.to_string(),
            error: error.to_string(),
        }
    }

    /// Get the channel name from an event
    pub fn channel(&self) -> &str {
        match self {
            Event::Message(msg) => &msg.channel,
            Event::Connect { channel, .. } => channel,
            Event::Disconnect { channel, .. } => channel,
            Event::Error { channel, .. } => channel,
        }
    }
}

/// Tool definition for LLM
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

impl ToolDefinition {
    /// Create a new tool definition
    pub fn new(name: &str, description: &str, parameters: Value) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            parameters,
        }
    }

    /// Convert to JSON for LLM API
    pub fn to_json(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": self.parameters
            }
        })
    }
}

/// Result of executing a tool
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub name: String,
    pub success: bool,
    pub content: String,
    pub error: Option<String>,
}

impl ToolResult {
    /// Create a successful result
    pub fn success(name: &str, content: &str) -> Self {
        Self {
            name: name.to_string(),
            success: true,
            content: content.to_string(),
            error: None,
        }
    }

    /// Create an error result
    pub fn error(name: &str, error: &str) -> Self {
        Self {
            name: name.to_string(),
            success: false,
            content: String::new(),
            error: Some(error.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_system() {
        let msg = Message::system("You are a helpful assistant");
        assert_eq!(msg.role, MessageRole::System);
        assert_eq!(msg.content, "You are a helpful assistant");
        assert!(msg.tool_calls.is_empty());
    }

    #[test]
    fn test_message_user() {
        let msg = Message::user("Hello, world!");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content, "Hello, world!");
    }

    #[test]
    fn test_message_assistant() {
        let msg = Message::assistant("I can help you.");
        assert_eq!(msg.role, MessageRole::Assistant);
        assert_eq!(msg.content, "I can help you.");
    }

    #[test]
    fn test_message_tool() {
        let msg = Message::tool("search result", "call_123", "web_search");
        assert_eq!(msg.role, MessageRole::Tool);
        assert_eq!(msg.content, "search result");
        assert_eq!(msg.tool_call_id, Some("call_123".to_string()));
        assert_eq!(msg.name, Some("web_search".to_string()));
    }

    #[test]
    fn test_message_to_json() {
        let msg = Message::user("Hello");
        let json = msg.to_json();
        assert_eq!(json["role"], "user");
        assert_eq!(json["content"], "Hello");
    }

    #[test]
    fn test_tool_call_new() {
        let tc = ToolCall::new("call_1", "read_file", json!({"path": "/test"}));
        assert_eq!(tc.id, "call_1");
        assert_eq!(tc.name, "read_file");
    }

    #[test]
    fn test_llm_response_empty() {
        let resp = LLMResponse::empty();
        assert!(resp.content.is_some());
        assert!(resp.tool_calls.is_empty());
        assert_eq!(resp.finish_reason, "stop");
    }

    #[test]
    fn test_inbound_message_new() {
        let msg = InboundMessage::new("telegram", "user123", "chat456", "Hello");
        assert_eq!(msg.channel, "telegram");
        assert_eq!(msg.sender_id, "user123");
        assert_eq!(msg.chat_id, "chat456");
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_inbound_message_session_key() {
        let msg = InboundMessage::new("telegram", "user123", "chat456", "Hello");
        assert_eq!(msg.session_key(), "telegram:chat456");
    }

    #[test]
    fn test_outbound_message_new() {
        let msg = OutboundMessage::new("telegram", "chat456", "Hello back!");
        assert_eq!(msg.channel, "telegram");
        assert_eq!(msg.chat_id, "chat456");
        assert_eq!(msg.content, "Hello back!");
    }

    #[test]
    fn test_outbound_message_reply() {
        let msg = OutboundMessage::reply("telegram", "chat456", "Reply", "original123");
        assert_eq!(msg.reply_to, Some("original123".to_string()));
    }

    #[test]
    fn test_tool_definition_new() {
        let def = ToolDefinition::new(
            "read_file",
            "Read a file",
            json!({"type": "object", "properties": {"path": {"type": "string"}}}),
        );
        assert_eq!(def.name, "read_file");
        assert_eq!(def.description, "Read a file");
    }

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult::success("read_file", "file content");
        assert!(result.success);
        assert_eq!(result.content, "file content");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_tool_result_error() {
        let result = ToolResult::error("read_file", "file not found");
        assert!(!result.success);
        assert!(result.content.is_empty());
        assert_eq!(result.error, Some("file not found".to_string()));
    }
}
