//! Tools module - external tools for the agent.
//!
//! # Available Tools
//!
//! - Web search (Brave Search API)
//! - Web fetch (URL content extraction)
//! - File operations (read, write, list)
//! - Command execution
//!
//! # Adding New Tools
//!
//! To add a new tool, implement the `Tool` trait and register it in the `ToolRegistry`.

pub mod web_search;
pub mod fetch;

pub use web_search::{BraveSearch, SearchResult};

use crate::types::ToolDefinition;
use serde_json::json;

/// Tool trait for extensibility
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    /// Tool name
    fn name(&self) -> &str;

    /// Tool description
    fn description(&self) -> &str;

    /// Get tool definition for LLM
    fn definition(&self) -> ToolDefinition;

    /// Execute the tool
    async fn execute(&self, args: &str) -> Result<String, String>;
}

/// Get all built-in tool definitions
pub fn get_builtin_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        crate::types::ToolDefinition::new(
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
        ),
        crate::types::ToolDefinition::new(
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
        ),
        crate::types::ToolDefinition::new(
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
        ),
        crate::types::ToolDefinition::new(
            "exec",
            "Execute a shell command and return the output.",
            json!({
                "type": "object",
                "properties": {
                    "cmd": {
                        "type": "string",
                        "description": "The command to execute"
                    }
                },
                "required": ["cmd"]
            }),
        ),
        crate::types::ToolDefinition::new(
            "web_search",
            "Search the web for information. Use this when you need current events.",
            json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query"
                    }
                },
                "required": ["query"]
            }),
        ),
        crate::types::ToolDefinition::new(
            "web_fetch",
            "Fetch and extract text content from a URL.",
            json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to fetch"
                    }
                },
                "required": ["url"]
            }),
        ),
    ]
}

/// Simple HTML to text extraction
pub fn extract_text(html: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;

    for c in html.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            text.push(c);
        }
    }

    // Clean up whitespace
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
