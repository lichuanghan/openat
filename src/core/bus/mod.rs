use crate::types::{Event, InboundMessage, OutboundMessage};
use tokio::sync::broadcast;

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
