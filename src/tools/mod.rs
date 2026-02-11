//! Tools module - external tools for the agent.
//!
//! # Available Tools
//!
//! - Web search (Brave Search API)
//! - Web fetch (URL content extraction)
//! - Shell execution (with safety guards)
//! - File operations (read, write, list)
//!
//! # Adding New Tools
//!
//! To add a new tool, implement the `Tool` trait and register it in the `ToolRegistry`.

pub mod web_search;
pub mod fetch;
pub mod shell;
pub mod filesystem;
pub mod cron_tool;
pub mod message;
pub mod spawn;
pub mod html;
pub mod macros;

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
        crate::types::ToolDefinition::new(
            "message",
            "Send a message to a user on a chat channel.",
            json!({
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "The message content to send"
                    },
                    "channel": {
                        "type": "string",
                        "description": "Optional: target channel"
                    },
                    "chat_id": {
                        "type": "string",
                        "description": "Optional: target chat/user ID"
                    }
                },
                "required": ["content"]
            }),
        ),
    ]
}
