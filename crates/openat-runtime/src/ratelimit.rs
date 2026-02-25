//! Rate limiting utilities

use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum number of requests allowed in the window
    pub max_requests: u32,
    /// Time window for rate limiting
    pub window: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 10,  // 10 requests
            window: Duration::from_secs(1),  // per second
        }
    }
}

/// Token bucket rate limiter for async contexts
#[derive(Clone)]
pub struct RateLimiter {
    config: RateLimitConfig,
    tokens: Arc<RwLock<TokenBucket>>,
}

struct TokenBucket {
    available: u32,
    last_refill: Instant,
}

impl RateLimiter {
    /// Create a new rate limiter with the given config
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config: config.clone(),
            tokens: Arc::new(RwLock::new(TokenBucket {
                available: config.max_requests,
                last_refill: Instant::now(),
            })),
        }
    }

    /// Try to acquire a token, returns true if successful
    pub async fn try_acquire(&self) -> bool {
        let mut bucket = self.tokens.write().await;
        bucket.refill(&self.config);

        if bucket.available > 0 {
            bucket.available -= 1;
            true
        } else {
            false
        }
    }

    /// Acquire a token, waiting if necessary
    pub async fn acquire(&self) {
        while !self.try_acquire().await {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// Wait until a token is available, then return the wait duration
    pub async fn acquire_with_wait(&self) -> Duration {
        let start = Instant::now();
        self.acquire().await;
        start.elapsed()
    }
}

impl TokenBucket {
    fn refill(&mut self, config: &RateLimitConfig) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);

        if elapsed >= config.window {
            // Reset tokens
            self.available = config.max_requests;
            self.last_refill = now;
        }
    }
}

/// Per-channel rate limiter that tracks messages per channel/chat
#[derive(Clone)]
pub struct ChannelRateLimiter {
    config: RateLimitConfig,
    channels: Arc<RwLock<std::collections::HashMap<String, ChannelBucket>>>,
}

struct ChannelBucket {
    timestamps: VecDeque<Instant>,
}

impl ChannelRateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            channels: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Check if a message can be sent to this channel, and record it if so
    pub async fn try_acquire(&self, channel_id: &str) -> bool {
        let mut channels = self.channels.write().await;
        let bucket = channels.entry(channel_id.to_string()).or_insert_with(|| ChannelBucket {
            timestamps: VecDeque::new(),
        });

        let now = Instant::now();

        // Remove old timestamps outside the window
        let window_start = now - self.config.window;
        while bucket.timestamps.front().map_or(false, |t| *t < window_start) {
            bucket.timestamps.pop_front();
        }

        // Check if we can send
        if bucket.timestamps.len() < self.config.max_requests as usize {
            bucket.timestamps.push_back(now);
            true
        } else {
            false
        }
    }

    /// Wait until a message can be sent to this channel
    pub async fn acquire(&self, channel_id: &str) {
        while !self.try_acquire(channel_id).await {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests: 2,
            window: Duration::from_secs(1),
        });

        // Should be able to acquire 2 immediately
        assert!(limiter.try_acquire().await);
        limiter.acquire().await;

        // Third one should fail
        assert!(!limiter.try_acquire().await);
    }

    #[tokio::test]
    async fn test_channel_rate_limiter() {
        let limiter = ChannelRateLimiter::new(RateLimitConfig {
            max_requests: 3,
            window: Duration::from_secs(1),
        });

        // Should allow 3 messages
        assert!(limiter.try_acquire("channel1").await);
        assert!(limiter.try_acquire("channel1").await);
        assert!(limiter.try_acquire("channel1").await);

        // Fourth should fail
        assert!(!limiter.try_acquire("channel1").await);

        // But channel2 should still work
        assert!(limiter.try_acquire("channel2").await);
    }
}
