//! Message tool for sending messages to users on chat channels.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::types::{OutboundMessage, ToolDefinition};
use crate::core::bus::MessageBus;

/// Tool to send messages to users on chat channels.
#[derive(Debug, Clone)]
pub struct MessageTool {
    bus: MessageBus,
    /// Default channel from session context
    default_channel: Option<String>,
    /// Default chat_id from session context
    default_chat_id: Option<String>,
}

impl MessageTool {
    pub fn new(bus: &MessageBus) -> Self {
        Self {
            bus: bus.clone(),
            default_channel: None,
            default_chat_id: None,
        }
    }

    /// Set the current session context
    pub fn set_context(&mut self, channel: String, chat_id: String) {
        self.default_channel = Some(channel);
        self.default_chat_id = Some(chat_id);
    }
}

#[async_trait]
impl crate::tools::Tool for MessageTool {
    fn name(&self) -> &str {
        "message"
    }

    fn description(&self) -> &str {
        "Send a message to the user. Use this when you want to communicate something to a user on a chat channel."
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            "message",
            "Send a message to a user on a chat channel.",
            json!({
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "The message content to send"
                    },
                    "channel": {
                        "type": "string",
                        "description": "Optional: target channel (telegram, whatsapp, etc.)"
                    },
                    "chat_id": {
                        "type": "string",
                        "description": "Optional: target chat/user ID"
                    }
                },
                "required": ["content"]
            }),
        )
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        #[derive(Deserialize)]
        struct Args {
            content: String,
            channel: Option<String>,
            chat_id: Option<String>,
        }

        let args: Args = serde_json::from_str(args)
            .map_err(|e| format!("Invalid arguments: {}", e))?;

        let channel = args.channel.or(self.default_channel.clone())
            .ok_or("Error: No target channel specified")?;
        let chat_id = args.chat_id.or(self.default_chat_id.clone())
            .ok_or("Error: No target chat_id specified")?;

        let msg = OutboundMessage::new(&channel, &chat_id, &args.content);

        self.bus.publish_outbound(msg).await;

        Ok(format!("Message sent to {}:{}", channel, chat_id))
    }
}
