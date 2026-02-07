//! Cron tool for scheduling reminders and tasks.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::types::ToolDefinition;
use crate::core::scheduler::{JobManager, ScheduledJob};

/// Cron tool for scheduling reminders and tasks
#[derive(Debug, Clone)]
pub struct CronTool {
    manager: JobManager,
    /// Session context for delivery
    channel: Option<String>,
    chat_id: Option<String>,
}

impl CronTool {
    pub fn new() -> Self {
        Self {
            manager: JobManager::new(),
            channel: None,
            chat_id: None,
        }
    }

    /// Set the current session context for delivery
    pub fn set_context(&mut self, channel: String, chat_id: String) {
        self.channel = Some(channel);
        self.chat_id = Some(chat_id);
    }
}

#[async_trait]
impl crate::tools::Tool for CronTool {
    fn name(&self) -> &str {
        "cron"
    }

    fn description(&self) -> &str {
        "Schedule reminders and recurring tasks. Actions: add, list, remove."
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            "cron",
            "Schedule reminders and recurring tasks. Actions: add, list, remove.",
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["add", "list", "remove"],
                        "description": "Action to perform: add, list, or remove"
                    },
                    "message": {
                        "type": "string",
                        "description": "Reminder message (for add action)"
                    },
                    "every_seconds": {
                        "type": "integer",
                        "description": "Interval in seconds for recurring tasks (for add action)"
                    },
                    "cron_expr": {
                        "type": "string",
                        "description": "Cron expression like '0 9 * * *' (for add action)"
                    },
                    "job_id": {
                        "type": "string",
                        "description": "Job ID (for remove action)"
                    }
                },
                "required": ["action"]
            }),
        )
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        #[derive(Deserialize)]
        struct Args {
            action: String,
            message: Option<String>,
            every_seconds: Option<u64>,
            cron_expr: Option<String>,
            job_id: Option<String>,
        }

        let args: Args = serde_json::from_str(args)
            .map_err(|e| format!("Invalid arguments: {}", e))?;

        match args.action.as_str() {
            "add" => self.add_job(
                args.message.unwrap_or_default(),
                args.every_seconds,
                args.cron_expr,
            ).await,
            "list" => self.list_jobs().await,
            "remove" => self.remove_job(args.job_id).await,
            _ => Err(format!("Unknown action: {}", args.action)),
        }
    }
}

impl CronTool {
    async fn add_job(
        &self,
        message: String,
        every_seconds: Option<u64>,
        cron_expr: Option<String>,
    ) -> Result<String, String> {
        if message.is_empty() {
            return Err("Error: message is required for add".to_string());
        }

        let channel = self.channel.clone().ok_or("Error: no session context (channel)")?;
        let chat_id = self.chat_id.clone().ok_or("Error: no session context (chat_id)")?;

        // Build schedule
        let (interval, cron) = if let Some(seconds) = every_seconds {
            (Some(seconds), None)
        } else if let Some(expr) = cron_expr {
            (None, Some(expr))
        } else {
            return Err("Error: either every_seconds or cron_expr is required".to_string());
        };

        let name: String = message.chars().take(30).collect();
        let job_name = name.clone();

        let mut job = ScheduledJob {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            message,
            enabled: true,
            interval_seconds: interval,
            cron_expression: cron,
            deliver_response: false,
            deliver_to: Some(chat_id),
            deliver_channel: Some(channel),
            created_at: chrono::Utc::now(),
            last_run: None,
            next_run: None,
        };

        let mut manager = self.manager.clone();
        let job_id = job.id.clone();
        manager.add_job(&mut job);

        Ok(format!("Created job '{}' (id: {})", job_name, job_id))
    }

    async fn list_jobs(&self) -> Result<String, String> {
        let jobs = self.manager.load_jobs();

        if jobs.is_empty() {
            return Ok("No scheduled jobs.".to_string());
        }

        let lines: Vec<String> = jobs
            .iter()
            .map(|j| {
                let schedule = if let Some(sec) = j.interval_seconds {
                    format!("every {}s", sec)
                } else if let Some(expr) = &j.cron_expression {
                    format!("cron: {}", expr)
                } else {
                    "one-time".to_string()
                };
                format!(
                    "- {} (id: {}, enabled: {}, {})",
                    j.name,
                    j.id,
                    if j.enabled { "yes" } else { "no" },
                    schedule
                )
            })
            .collect();

        Ok(format!("Scheduled jobs:\n{}", lines.join("\n")))
    }

    async fn remove_job(&self, job_id: Option<String>) -> Result<String, String> {
        let job_id = job_id.ok_or("Error: job_id is required for remove".to_string())?;

        let deleted = self.manager.delete_job(&job_id);

        if deleted {
            Ok(format!("Removed job {}", job_id))
        } else {
            Err(format!("Job {} not found", job_id))
        }
    }
}

impl Default for CronTool {
    fn default() -> Self {
        Self::new()
    }
}
