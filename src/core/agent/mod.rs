//! Agent core module.
//!
//! This module provides:
//! - `AgentExecutor`: Full-featured agent with message bus integration
//! - `SimpleAgent`: Lightweight agent for CLI usage

pub mod executor;
pub mod simple;
pub mod skills;
pub mod memory;

pub use executor::AgentExecutor;
pub use simple::SimpleAgent;
