use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Heartbeat manager for monitoring agent health
#[derive(Debug)]
pub struct Heartbeat {
    running: AtomicBool,
    last_heartbeat: AtomicU64,
    start_time: Instant,
}

impl Heartbeat {
    pub fn new() -> Self {
        Self {
            running: AtomicBool::new(false),
            last_heartbeat: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    /// Start the heartbeat
    pub fn start(&self) {
        self.running.store(true, Ordering::SeqCst);
        self.last_heartbeat.store(Self::now_millis(), Ordering::SeqCst);
    }

    /// Stop the heartbeat
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Record a heartbeat
    pub fn beat(&self) {
        self.last_heartbeat.store(Self::now_millis(), Ordering::SeqCst);
    }

    /// Check if still alive
    pub fn is_alive(&self, timeout_millis: u64) -> bool {
        let last = self.last_heartbeat.load(Ordering::SeqCst);
        let now = Self::now_millis();
        now - last < timeout_millis
    }

    /// Get uptime in seconds
    pub fn uptime(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    fn now_millis() -> u64 {
        std::time::SystemTime::UNIX_EPOCH
            .elapsed()
            .unwrap_or(Duration::ZERO)
            .as_millis() as u64
    }
}

impl Default for Heartbeat {
    fn default() -> Self {
        Self::new()
    }
}
