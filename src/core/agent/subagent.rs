//! Subagent manager for background task execution.
//!
//! Subagents are lightweight agent instances that run in the background.

use crate::types::InboundMessage;
use crate::llm::LLMProvider;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, error, info};
use uuid::Uuid;

/// Subagent configuration
#[derive(Debug, Clone)]
pub struct SubagentConfig {
    pub workspace: PathBuf,
    pub max_iterations: usize,
    pub timeout_seconds: u64,
}

impl Default for SubagentConfig {
    fn default() -> Self {
        Self {
            workspace: PathBuf::from("."),
            max_iterations: 15,
            timeout_seconds: 300,
        }
    }
}

/// Subagent task result
#[derive(Debug, Clone)]
pub struct SubagentResult {
    pub task_id: String,
    pub label: String,
    pub result: String,
    pub success: bool,
}

/// Subagent manager
#[derive(Debug, Clone)]
pub struct SubagentManager {
    config: SubagentConfig,
    bus: Arc<()>, // Placeholder for bus
}

impl SubagentManager {
    /// Create a new subagent manager
    pub fn new(_bus: &(), config: Option<SubagentConfig>) -> Self {
        Self {
            config: config.unwrap_or_default(),
            bus: Arc::new(()),
        }
    }

    /// Spawn a subagent to execute a task
    pub async fn spawn(
        &self,
        task: String,
        label: Option<String>,
        _origin_channel: String,
        _origin_chat_id: String,
    ) -> Result<String, String> {
        let task_id = format!("sub-{}", Uuid::new_v4().to_string()[..8].to_string());
        let display_label = label.unwrap_or_else(|| {
            if task.len() > 30 {
                format!("{}...", &task[..30])
            } else {
                task.clone()
            }
        });

        info!("Subagent [{}]: {}", task_id, display_label);

        // Simplified - just return info message
        Ok(format!("Subagent [{}] '{}' spawned.", task_id, display_label))
    }

    /// Get number of running subagents
    pub fn running_count(&self) -> usize {
        0
    }
}
