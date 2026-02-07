//! Message bus for decoupled channel-agent communication.
//!
//! Provides async channels for inbound/outbound messages and events.

use crate::types::{Event, InboundMessage, OutboundMessage};
use tokio::sync::broadcast;
use tracing::{debug, info};

/// Async message bus for decoupled channel-agent communication
#[derive(Debug, Clone)]
pub struct MessageBus {
    inbound_tx: broadcast::Sender<InboundMessage>,
    outbound_tx: broadcast::Sender<OutboundMessage>,
    events_tx: broadcast::Sender<Event>,
}

impl MessageBus {
    pub fn new() -> Self {
        let (inbound_tx, _) = broadcast::channel(100);
        let (outbound_tx, _) = broadcast::channel(100);
        let (events_tx, _) = broadcast::channel(50);

        Self {
            inbound_tx,
            outbound_tx,
            events_tx,
        }
    }

    // ============ Inbound Messages ============

    /// Publish an inbound message (from channels to agent)
    pub async fn publish_inbound(&self, msg: InboundMessage) {
        let _ = self.inbound_tx.send(msg);
    }

    /// Subscribe to inbound messages
    pub fn subscribe_inbound(&self) -> broadcast::Receiver<InboundMessage> {
        self.inbound_tx.subscribe()
    }

    // ============ Outbound Messages ============

    /// Publish an outbound message (from agent to channels)
    pub async fn publish_outbound(&self, msg: OutboundMessage) {
        let _ = self.outbound_tx.send(msg);
    }

    /// Subscribe to outbound messages
    pub fn subscribe_outbound(&self) -> broadcast::Receiver<OutboundMessage> {
        self.outbound_tx.subscribe()
    }

    // ============ Events ============

    /// Publish a system event
    pub async fn publish_event(&self, event: Event) {
        let channel_name = match &event {
            Event::Message(msg) => msg.channel.clone(),
            Event::Connect { channel, .. } => channel.clone(),
            Event::Disconnect { channel, .. } => channel.clone(),
            Event::Error { channel, .. } => channel.clone(),
        };

        debug!("Publishing event: {:?}", event);
        let _ = self.events_tx.send(event);
    }

    /// Subscribe to system events
    pub fn subscribe_events(&self) -> broadcast::Receiver<Event> {
        self.events_tx.subscribe()
    }

    // ============ Convenience Methods ============

    /// Publish a connection event
    pub async fn publish_connect(&self, channel: &str, chat_id: &str) {
        self.publish_event(Event::connect(channel, chat_id)).await;
    }

    /// Publish a disconnection event
    pub async fn publish_disconnect(&self, channel: &str, chat_id: &str) {
        self.publish_event(Event::disconnect(channel, chat_id)).await;
    }

    /// Publish an error event
    pub async fn publish_error(&self, channel: &str, error: &str) {
        self.publish_event(Event::error(channel, error)).await;
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::InboundMessage;

    #[tokio::test]
    async fn test_message_bus_inbound() {
        let bus = MessageBus::new();
        let msg = InboundMessage::new("telegram", "user123", "chat456", "Hello");

        // Subscribe first to avoid missing message
        let mut rx = bus.subscribe_inbound();
        bus.publish_inbound(msg.clone()).await;

        let received = rx.recv().await.unwrap();

        assert_eq!(received.channel, "telegram");
        assert_eq!(received.sender_id, "user123");
        assert_eq!(received.content, "Hello");
    }

    #[tokio::test]
    async fn test_message_bus_outbound() {
        let bus = MessageBus::new();
        let msg = OutboundMessage::new("telegram", "chat456", "Response");

        // Subscribe first to avoid missing message
        let mut rx = bus.subscribe_outbound();
        bus.publish_outbound(msg.clone()).await;

        let received = rx.recv().await.unwrap();

        assert_eq!(received.channel, "telegram");
        assert_eq!(received.content, "Response");
    }

    #[tokio::test]
    async fn test_message_bus_events() {
        let bus = MessageBus::new();

        // Subscribe first to avoid missing event
        let mut rx = bus.subscribe_events();
        bus.publish_connect("telegram", "chat123").await;

        let received = rx.recv().await.unwrap();

        match received {
            Event::Connect { channel, chat_id } => {
                assert_eq!(channel, "telegram");
                assert_eq!(chat_id, "chat123");
            }
            _ => panic!("Expected Connect event"),
        }
    }

    #[tokio::test]
    async fn test_message_bus_error_event() {
        let bus = MessageBus::new();

        // Subscribe first to avoid missing event
        let mut rx = bus.subscribe_events();
        bus.publish_error("telegram", "connection lost").await;

        let received = rx.recv().await.unwrap();

        match received {
            Event::Error { channel, error } => {
                assert_eq!(channel, "telegram");
                assert_eq!(error, "connection lost");
            }
            _ => panic!("Expected Error event"),
        }
    }

    #[test]
    fn test_message_bus_default() {
        let bus = MessageBus::default();
        assert!(!format!("{:?}", bus).is_empty());
    }
}
