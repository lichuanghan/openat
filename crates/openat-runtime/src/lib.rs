//! Runtime core for openat
//!
//! Provides MessageBus for inter-component communication,
//! lifecycle management, and shutdown coordination.

pub mod bus;
pub mod lifecycle;
pub mod ratelimit;

pub use bus::MessageBus;
pub use lifecycle::{Lifecycle, ShutdownSignal};
pub use ratelimit::{ChannelRateLimiter, RateLimitConfig, RateLimiter};
