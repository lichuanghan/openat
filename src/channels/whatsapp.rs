use crate::bus::{InboundMessage, MessageBus};
use crate::config::Config;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, WebSocketStream};
use tracing::{info, warn};
use futures_util::stream::StreamExt;
use futures_util::sink::SinkExt;

/// WhatsApp bridge message format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppBridgeMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub content: Option<String>,
    pub sender: Option<String>,
    pub sender_name: Option<String>,
    pub chat_id: Option<String>,
    pub timestamp: Option<u64>,
}

/// WhatsApp channel implementation using WebSocket bridge
#[derive(Debug)]
pub struct WhatsAppChannel {
    bus: MessageBus,
    bridge_url: String,
    phone_number: Option<String>,
    allowed_numbers: Vec<String>,
    ws_stream: Option<WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>>,
    running: bool,
}

impl WhatsAppChannel {
    pub fn new(config: &Config, bus: MessageBus) -> Self {
        Self {
            bus,
            bridge_url: config.channels.whatsapp.bridge_url.clone(),
            phone_number: config.channels.whatsapp.phone_number.clone(),
            allowed_numbers: config.channels.whatsapp.allowed_numbers
                .iter()
                .cloned()
                .collect(),
            ws_stream: None,
            running: false,
        }
    }

    fn is_allowed(&self, sender: &str) -> bool {
        self.allowed_numbers.is_empty() || self.allowed_numbers.contains(&sender.to_string())
    }

    /// Connect to WhatsApp bridge
    pub async fn connect(&mut self) -> Result<()> {
        info!("Connecting to WhatsApp bridge at {}...", self.bridge_url);

        let (ws_stream, _) = connect_async(&self.bridge_url as &str)
            .await
            .context("Failed to connect to WhatsApp bridge")?;

        self.ws_stream = Some(ws_stream);
        self.running = true;

        info!("Connected to WhatsApp bridge");
        Ok(())
    }

    /// Start listening for messages
    pub async fn run(&mut self) {
        if self.ws_stream.is_none() {
            if let Err(e) = self.connect().await {
                warn!("Failed to connect to WhatsApp bridge: {}", e);
                return;
            }
        }

        info!("WhatsApp channel started");

        while self.running {
            if let Some(stream) = self.ws_stream.as_mut() {
                tokio::select! {
                    msg = stream.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                self.handle_message(&text).await;
                            }
                            Some(Ok(Message::Binary(data))) => {
                                if let Ok(text) = String::from_utf8(data) {
                                    self.handle_message(&text).await;
                                }
                            }
                            Some(Err(e)) => {
                                warn!("WebSocket error: {}", e);
                                // Try to reconnect
                                if let Err(e) = self.connect().await {
                                    warn!("Reconnection failed: {}", e);
                                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                                }
                            }
                            None => {
                                warn!("WebSocket stream ended");
                                self.running = false;
                            }
                            _ => {}
                        }
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
                        // Keep-alive ping
                    }
                }
            }
        }
    }

    async fn handle_message(&self, text: &str) {
        // Try to parse as WhatsApp bridge message
        if let Ok(msg) = serde_json::from_str::<WhatsAppBridgeMessage>(text) {
            if msg.msg_type != "message" {
                return;
            }

            let sender = match &msg.sender {
                Some(s) => s.clone(),
                None => return,
            };

            // Check if sender is allowed
            if !self.is_allowed(&sender) {
                warn!("Sender {} is not allowed", sender);
                return;
            }

            let chat_id = msg.chat_id.as_ref()
                .unwrap_or(&sender)
                .clone();

            let content = msg.content.as_ref()
                .map(|s| s.clone())
                .unwrap_or_default();

            if content.is_empty() {
                return;
            }

            info!("Received WhatsApp message from {}: {}", sender, content);

            // Create and publish inbound message
            let inbound = InboundMessage::new(
                "whatsapp",
                &sender,
                &chat_id,
                &content,
            );

            self.bus.publish_inbound(inbound).await;
        }
    }

    /// Send a message through WhatsApp bridge
    pub async fn send_message(&mut self, chat_id: &str, content: &str) -> Result<()> {
        if self.ws_stream.is_none() {
            return Err(anyhow::anyhow!("Not connected to WhatsApp bridge"));
        }

        let message = serde_json::json!({
            "type": "message",
            "content": content,
            "chat_id": chat_id
        });

        if let Some(stream) = self.ws_stream.as_mut() {
            stream.send(Message::Text(message.to_string()))
                .await
                .context("Failed to send message")?;
        }

        Ok(())
    }

    pub fn stop(&mut self) {
        self.running = false;
    }
}

/// WhatsApp outbound handler
pub struct WhatsAppOutboundHandler {
    channel: std::sync::Arc<std::sync::Mutex<WhatsAppChannel>>,
}

impl WhatsAppOutboundHandler {
    pub fn new(channel: std::sync::Arc<std::sync::Mutex<WhatsAppChannel>>) -> Self {
        Self { channel }
    }

    pub async fn handle_outbound(&self, chat_id: &str, content: &str) -> Result<()> {
        let mut channel = self.channel.lock().unwrap();
        channel.send_message(chat_id, content).await
    }
}
