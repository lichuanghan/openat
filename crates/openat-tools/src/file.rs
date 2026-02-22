//! File system tools

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use crate::{Tool, ToolDefinition};

/// File system tool
#[derive(Debug, Clone)]
pub struct FileTool {
    allowed_dir: Option<PathBuf>,
}

impl FileTool {
    pub fn new(allowed_dir: Option<PathBuf>) -> Self {
        Self { allowed_dir }
    }

    fn resolve_path(&self, path: &str) -> Result<PathBuf, String> {
        let path = if path.starts_with("~") {
            if let Ok(home) = std::env::var("HOME") {
                path.replace("~", &home)
            } else {
                path.to_string()
            }
        } else {
            path.to_string()
        };

        let resolved = PathBuf::from(&path)
            .canonicalize()
            .map_err(|_| format!("Path not found: {}", path))?;

        if let Some(dir) = &self.allowed_dir {
            let allowed = dir.canonicalize().map_err(|_| "Invalid allowed directory")?;
            if !resolved.starts_with(&allowed) {
                return Err(format!("Path {} is outside allowed directory", path));
            }
        }

        Ok(resolved)
    }
}

#[async_trait]
impl Tool for FileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a file at the given path."
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            "read_file",
            "Read the contents of a file at the given path.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "The file path to read" }
                },
                "required": ["path"]
            }),
        )
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        #[derive(Deserialize)]
        struct Args {
            path: String,
        }

        let args: Args = serde_json::from_str(args)
            .map_err(|e| format!("Invalid arguments: {}", e))?;

        let file_path = self.resolve_path(&args.path)?;

        if !file_path.exists() {
            return Err(format!("File not found: {}", args.path));
        }

        if !file_path.is_file() {
            return Err(format!("Not a file: {}", args.path));
        }

        tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| format!("Error reading file: {}", e))
    }
}
