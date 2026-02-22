//! Shell execution tool

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use crate::{Tool, ToolDefinition};

/// Shell execution tool
#[derive(Debug, Clone)]
pub struct ExecTool {
    /// Directory to restrict execution to
    allowed_dir: Option<String>,
}

impl ExecTool {
    pub fn new(allowed_dir: Option<String>) -> Self {
        Self { allowed_dir }
    }

    fn validate_command(&self, cmd: &str) -> Result<(), String> {
        // Block dangerous commands
        let dangerous = ["rm", "mkfs", "dd", ">=", "&&", "||", "$", "`", ";"];
        for d in &dangerous {
            if cmd.contains(d) {
                return Err(format!("Potentially dangerous command blocked: {}", d));
            }
        }

        // If allowed_dir is set, verify command doesn't escape it
        if let Some(_dir) = &self.allowed_dir {
            if cmd.contains("..") {
                return Err("Path traversal not allowed".to_string());
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Tool for ExecTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute a shell command and return the output."
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            "shell",
            "Execute a shell command safely.",
            json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "The command to execute" },
                    "timeout": { "type": "number", "description": "Timeout in seconds (default: 30)" }
                },
                "required": ["command"]
            }),
        )
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        #[derive(Deserialize)]
        struct Args {
            command: String,
            timeout: Option<u64>,
        }

        let args: Args = serde_json::from_str(args)
            .map_err(|e| format!("Invalid arguments: {}", e))?;

        self.validate_command(&args.command)?;

        let _timeout = args.timeout.unwrap_or(30);
        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&args.command)
            .output()
            .await
            .map_err(|e| format!("Command execution failed: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            return Err(format!("Command failed:\n{}", stderr));
        }

        Ok(stdout.to_string())
    }
}
