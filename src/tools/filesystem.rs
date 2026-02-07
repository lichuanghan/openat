//! File system tools: read, write, edit, list.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;

use crate::types::ToolDefinition;

/// Resolve path with optional directory restriction
fn resolve_path(path: &str, allowed_dir: Option<&PathBuf>) -> Result<PathBuf, String> {
    let resolved = PathBuf::from(path)
        .canonicalize()
        .map_err(|_| format!("Path not found: {}", path))?;

    if let Some(dir) = allowed_dir {
        let allowed = dir.canonicalize().map_err(|_| "Invalid allowed directory")?;
        if !resolved.starts_with(&allowed) {
            return Err(format!(
                "Path {} is outside allowed directory",
                path
            ));
        }
    }

    Ok(resolved)
}

/// Read file tool
#[derive(Debug, Clone)]
pub struct ReadFileTool {
    allowed_dir: Option<PathBuf>,
}

impl ReadFileTool {
    pub fn new(allowed_dir: Option<PathBuf>) -> Self {
        Self { allowed_dir }
    }
}

#[async_trait]
impl crate::tools::Tool for ReadFileTool {
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
                    "path": {
                        "type": "string",
                        "description": "The file path to read"
                    }
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

        let file_path = resolve_path(&args.path, self.allowed_dir.as_ref())?;

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

/// Write file tool
#[derive(Debug, Clone)]
pub struct WriteFileTool {
    allowed_dir: Option<PathBuf>,
}

impl WriteFileTool {
    pub fn new(allowed_dir: Option<PathBuf>) -> Self {
        Self { allowed_dir }
    }
}

#[async_trait]
impl crate::tools::Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write content to a file at the given path. Creates parent directories if needed."
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            "write_file",
            "Write content to a file. Creates parent directories if needed.",
            json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The file path to write to"
                    },
                    "content": {
                        "type": "string",
                        "description": "The content to write"
                    }
                },
                "required": ["path", "content"]
            }),
        )
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        #[derive(Deserialize)]
        struct Args {
            path: String,
            content: String,
        }

        let args: Args = serde_json::from_str(args)
            .map_err(|e| format!("Invalid arguments: {}", e))?;

        let file_path = resolve_path(&args.path, self.allowed_dir.as_ref())?;

        // Create parent directories
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Error creating directories: {}", e))?;
        }

        tokio::fs::write(&file_path, &args.content)
            .await
            .map_err(|e| format!("Error writing file: {}", e))?;

        Ok(format!(
            "Successfully wrote {} bytes to {}",
            args.content.len(),
            args.path
        ))
    }
}

/// Edit file tool (replace text)
#[derive(Debug, Clone)]
pub struct EditFileTool {
    allowed_dir: Option<PathBuf>,
}

impl EditFileTool {
    pub fn new(allowed_dir: Option<PathBuf>) -> Self {
        Self { allowed_dir }
    }
}

#[async_trait]
impl crate::tools::Tool for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Edit a file by replacing old_text with new_text. The old_text must exist exactly in the file."
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            "edit_file",
            "Edit a file by replacing old_text with new_text.",
            json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The file path to edit"
                    },
                    "old_text": {
                        "type": "string",
                        "description": "The exact text to find and replace"
                    },
                    "new_text": {
                        "type": "string",
                        "description": "The text to replace with"
                    }
                },
                "required": ["path", "old_text", "new_text"]
            }),
        )
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        #[derive(Deserialize)]
        struct Args {
            path: String,
            old_text: String,
            new_text: String,
        }

        let args: Args = serde_json::from_str(args)
            .map_err(|e| format!("Invalid arguments: {}", e))?;

        let file_path = resolve_path(&args.path, self.allowed_dir.as_ref())?;

        if !file_path.exists() {
            return Err(format!("File not found: {}", args.path));
        }

        let content = tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| format!("Error reading file: {}", e))?;

        if !content.contains(&args.old_text) {
            return Err("old_text not found in file. Make sure it matches exactly.".to_string());
        }

        // Check for multiple occurrences
        let count = content.matches(&args.old_text).count();
        if count > 1 {
            return Err(format!(
                "Warning: old_text appears {} times. Please provide more context.",
                count
            ));
        }

        let new_content = content.replace(&args.old_text, &args.new_text);

        tokio::fs::write(&file_path, &new_content)
            .await
            .map_err(|e| format!("Error writing file: {}", e))?;

        Ok(format!("Successfully edited {}", args.path))
    }
}

/// List directory tool
#[derive(Debug, Clone)]
pub struct ListDirTool {
    allowed_dir: Option<PathBuf>,
}

impl ListDirTool {
    pub fn new(allowed_dir: Option<PathBuf>) -> Self {
        Self { allowed_dir }
    }
}

#[async_trait]
impl crate::tools::Tool for ListDirTool {
    fn name(&self) -> &str {
        "list_dir"
    }

    fn description(&self) -> &str {
        "List the contents of a directory."
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            "list_dir",
            "List the contents of a directory.",
            json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The directory path to list"
                    }
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

        let dir_path = resolve_path(&args.path, self.allowed_dir.as_ref())?;

        if !dir_path.exists() {
            return Err(format!("Directory not found: {}", args.path));
        }

        if !dir_path.is_dir() {
            return Err(format!("Not a directory: {}", args.path));
        }

        let mut entries: Vec<String> = Vec::new();

        let mut entries_iter = tokio::fs::read_dir(&dir_path)
            .await
            .map_err(|e| format!("Error reading directory: {}", e))?;

        while let Some(entry) = entries_iter.next_entry().await.map_err(|e| format!("Error reading entry: {}", e))? {
            let path = entry.path();
            let is_dir = path.is_dir();
            let name = entry.file_name().to_string_lossy().into_owned();

            if is_dir {
                entries.push(format!("📁 {}", name));
            } else {
                entries.push(format!("📄 {}", name));
            }
        }

        entries.sort();

        if entries.is_empty() {
            return Ok(format!("Directory {} is empty", args.path));
        }

        Ok(entries.join("\n"))
    }
}
