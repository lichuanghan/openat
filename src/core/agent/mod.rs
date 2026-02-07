//! Agent core module.
//!
//! This module provides:
//! - `AgentExecutor`: Full-featured agent with message bus integration
//! - `SimpleAgent`: Lightweight agent for CLI usage
//! - `ContextBuilder`: System prompt builder from bootstrap files, memory, and skills
//! - `SubagentManager`: Background subagent execution

pub mod executor;
pub mod simple;
pub mod skills;
pub mod memory;
pub mod context;
pub mod subagent;

pub use executor::AgentExecutor;
pub use simple::SimpleAgent;
pub use context::ContextBuilder;
pub use subagent::{SubagentManager, SubagentConfig, SubagentResult};
