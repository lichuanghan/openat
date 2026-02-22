//! Lifecycle management

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::signal;

/// Lifecycle controller for components
#[derive(Clone, Default)]
pub struct Lifecycle {
    running: Arc<AtomicBool>,
    shutdown_reason: Arc<std::sync::Mutex<Option<String>>>,
}

impl Lifecycle {
    /// Create a new lifecycle controller
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            shutdown_reason: Arc::new(Mutex::new(None)),
        }
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Start the component
    pub fn start(&self) {
        self.running.store(true, Ordering::SeqCst);
    }

    /// Stop the component
    pub fn stop(&self, reason: &str) {
        self.running.store(false, Ordering::SeqCst);
        *self.shutdown_reason.lock().unwrap() = Some(reason.to_string());
    }

    /// Get shutdown reason
    pub fn shutdown_reason(&self) -> Option<String> {
        self.shutdown_reason.lock().unwrap().clone()
    }
}

/// Shutdown signal handler
pub struct ShutdownSignal {
    lifecycle: Lifecycle,
}

impl ShutdownSignal {
    /// Create a new shutdown signal handler
    pub fn new(lifecycle: Lifecycle) -> Self {
        Self { lifecycle }
    }

    /// Wait for shutdown signal (Ctrl+C)
    pub async fn wait(&self) {
        let _ = signal::ctrl_c().await;
        self.lifecycle.stop("ctrl_c");
    }
}

use std::sync::Mutex;
