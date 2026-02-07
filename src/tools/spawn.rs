//! Spawn tool for creating background subagents.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::types::ToolDefinition;
use crate::core::agent::SubagentManager;

/// Tool to spawn a subagent for background task execution.
#[derive(Debug, Clone)]
pub struct SpawnTool {
    manager: SubagentManager,
    /// Session context for delivery
    channel: Option<String>,
    chat_id: Option<String>,
}

impl SpawnTool {
    pub fn new(manager: SubagentManager) -> Self {
        Self {
            manager,
            channel: None,
            chat_id: None,
        }
    }

    /// Set the origin context for subagent announcements
    pub fn set_context(&mut self, channel: String, chat_id: String) {
        self.channel = Some(channel);
        self.chat_id = Some(chat_id);
    }
}

#[async_trait]
impl crate::tools::Tool for SpawnTool {
    fn name(&self) -> &str {
        "spawn"
    }

    fn description(&self) -> &str {
        "Spawn a subagent to handle a task in the background. \
        Use this for complex or time-consuming tasks that can run independently. \
        The subagent will complete the task and report back when done."
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            "spawn",
            "Spawn a subagent for background task execution.",
            json!({
                "type": "object",
                "properties": {
                    "task": {
                        "type": "string",
                        "description": "The task for the subagent to complete"
                    },
                    "label": {
                        "type": "string",
                        "description": "Optional short label for the task"
                    }
                },
                "required": ["task"]
            }),
        )
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        #[derive(Deserialize)]
        struct Args {
            task: String,
            label: Option<String>,
        }

        let args: Args = serde_json::from_str(args)
            .map_err(|e| format!("Invalid arguments: {}", e))?;

        let channel = self.channel.clone().unwrap_or_else(|| "cli".to_string());
        let chat_id = self.chat_id.clone().unwrap_or_else(|| "direct".to_string());

        self.manager.spawn(
            args.task,
            args.label,
            channel,
            chat_id,
        ).await
    }
}
