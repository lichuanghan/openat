//! Shell execution tool.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::types::ToolDefinition;

/// Shell execution tool with safety guards.
#[derive(Debug, Clone)]
pub struct ShellTool {
    /// Timeout in seconds
    timeout: u64,
    /// Working directory
    working_dir: Option<String>,
    /// Maximum output length
    max_output: usize,
}

impl ShellTool {
    pub fn new(timeout: u64, working_dir: Option<String>) -> Self {
        Self {
            timeout,
            working_dir,
            max_output: 10000,
        }
    }

    /// Check if command contains dangerous patterns
    fn guard_command(&self, command: &str) -> Option<String> {
        let cmd = command.trim().to_lowercase();

        // Dangerous patterns that should be blocked
        let deny_patterns = [
            r"\brm\s+-[rf]{1,2}\b",        // rm -r, rm -rf, rm -fr
            r"\bdel\s+/[fq]\b",             // del /f, del /q (Windows)
            r"\brmdir\s+/s\b",              // rmdir /s (Windows)
            r"\b(format|mkfs|diskpart)\b",  // disk formatting
            r"\bdd\s+if=",                  // dd disk operations
            r">\s*/dev/sd",                 // write to disk devices
            r">\s*/dev/nvme",               // write to nvme devices
            r"\b(shutdown|reboot|poweroff)\b", // system power commands
            r":\(\)\s*\{.*\};\s*:",         // fork bomb
            r"\bsudo\s+su\b",               // sudo to root
            r"\bchmod\s+777\b",             // overly permissive permissions
            r"\bchown\s+.*:\s*root",        // chown to root
        ];

        for pattern in &deny_patterns {
            if regex::Regex::new(pattern)
                .unwrap()
                .is_match(&cmd)
            {
                return Some("Error: Command blocked by safety guard (dangerous pattern detected)".to_string());
            }
        }

        None
    }
}

#[async_trait]
impl crate::tools::Tool for ShellTool {
    fn name(&self) -> &str {
        "exec"
    }

    fn description(&self) -> &str {
        "Execute a shell command and return its output. Use with caution - dangerous commands are blocked."
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            "exec",
            "Execute a shell command and return its output. Use with caution - dangerous commands are blocked.",
            json!({
                "type": "object",
                "properties": {
                    "cmd": {
                        "type": "string",
                        "description": "The shell command to execute"
                    },
                    "working_dir": {
                        "type": "string",
                        "description": "Optional working directory for the command"
                    }
                },
                "required": ["cmd"]
            }),
        )
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        #[derive(Deserialize)]
        struct Args {
            cmd: String,
            working_dir: Option<String>,
        }

        let args: Args = serde_json::from_str(args)
            .map_err(|e| format!("Invalid arguments: {}", e))?;

        // Safety guard
        if let Some(error) = self.guard_command(&args.cmd) {
            return Err(error);
        }

        let cwd = args.working_dir.or(self.working_dir.clone());

        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&args.cmd)
            .current_dir(cwd.unwrap_or_else(|| ".".to_string()))
            .kill_on_drop(true)
            .output()
            .await
            .map_err(|e| format!("Failed to execute command: {}", e))?;

        let mut result = String::new();

        if !output.stdout.is_empty() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            result.push_str(&stdout);
        }

        if !output.stderr.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.trim().is_empty() {
                if !result.is_empty() {
                    result.push_str("\nSTDERR:\n");
                }
                result.push_str(&stderr);
            }
        }

        if output.status.code() != Some(0) {
            let code = output.status.code().unwrap_or(-1);
            if !result.is_empty() {
                result.push_str("\n");
            }
            result.push_str(&format!("Exit code: {}", code));
        }

        if result.is_empty() {
            result = "(no output)".to_string();
        }

        // Truncate very long output
        if result.len() > self.max_output {
            let truncated = result.len() - self.max_output;
            result.truncate(self.max_output);
            result.push_str(&format!("\n... (truncated, {} more chars)", truncated));
        }

        Ok(result)
    }
}
