//! MessageBus for inter-component communication

use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info};
use openat_types::{InboundMessage, OutboundMessage};

/// Message bus for communication between components
///
/// # Usage
///
/// ```
/// use openat_runtime::MessageBus;
///
/// let bus = MessageBus::new();
/// ```
#[derive(Clone)]
pub struct MessageBus {
    /// Channel for inbound messages (channel -> agent)
    inbound_tx: broadcast::Sender<Arc<InboundMessage>>,

    /// Channel for outbound messages (agent -> channel)
    outbound_tx: broadcast::Sender<Arc<OutboundMessage>>,

    /// Event bus for system events
    events: Arc<RwLock<Vec<SystemEvent>>>,
}

#[derive(Debug, Clone)]
pub enum SystemEvent {
    Connected { channel: String },
    Disconnected { channel: String, reason: String },
    Error { channel: String, error: String },
    MessageReceived { channel: String, sender: String },
    MessageSent { channel: String },
}

impl MessageBus {
    /// Create a new message bus
    pub fn new() -> Self {
        let (inbound_tx, _) = broadcast::channel(100);
        let (outbound_tx, _) = broadcast::channel(100);

        Self {
            inbound_tx,
            outbound_tx,
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Subscribe to inbound messages
    pub fn subscribe_inbound(&self) -> broadcast::Receiver<Arc<InboundMessage>> {
        self.inbound_tx.subscribe()
    }

    /// Subscribe to outbound messages
    pub fn subscribe_outbound(&self) -> broadcast::Receiver<Arc<OutboundMessage>> {
        self.outbound_tx.subscribe()
    }

    /// Publish an inbound message
    pub async fn publish_inbound(&self, message: InboundMessage) {
        let msg = Arc::new(message);
        if let Err(e) = self.inbound_tx.send(msg.clone()) {
            debug!("No subscribers for inbound message: {}", e);
        } else {
            info!("Published inbound message: {}", msg.content);
        }
    }

    /// Publish an outbound message
    pub async fn publish_outbound(&self, message: OutboundMessage) {
        let msg = Arc::new(message);
        if let Err(e) = self.outbound_tx.send(msg.clone()) {
            debug!("No subscribers for outbound message: {}", e);
        } else {
            info!("Published outbound message: {}", msg.content);
        }
    }

    /// Publish an event
    pub async fn publish_event(&self, event: SystemEvent) {
        let mut events = self.events.write().await;
        events.push(event);
        // Keep only last 100 events
        if events.len() > 100 {
            events.remove(0);
        }
    }

    /// Get recent events
    pub async fn recent_events(&self) -> Vec<SystemEvent> {
        self.events.read().await.clone()
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}
