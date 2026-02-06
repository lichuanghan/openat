//! Simple CLI Agent - standalone agent for interactive use.
//!
//! This module provides a simple `SimpleAgent` that can be used directly
//! from CLI without requiring the MessageBus infrastructure.
//!
//! For full-featured agent with message bus integration, use `AgentExecutor`.

use crate::llm::LLMProvider;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, info};

/// Simple agent for CLI usage - no message bus required
pub struct SimpleAgent {
    provider: Box<dyn LLMProvider>,
    model: String,
    workspace: PathBuf,
}

impl SimpleAgent {
    /// Create a new simple agent
    pub fn new(provider: Box<dyn LLMProvider>, model: String, workspace: PathBuf) -> Self {
        debug!("Creating SimpleAgent with model: {}", model);
        Self {
            provider,
            model,
            workspace,
        }
    }

    /// Chat with the agent
    pub async fn chat(&self, message: &str) -> String {
        info!("Processing message: {}...", message.chars().take(50).collect::<String>());

        let mut messages = vec![
            json!({
                "role": "system",
                "content": self.system_prompt()
            }),
        ];

        messages.push(json!({
            "role": "user",
            "content": message
        }));

        let mut iterations = 0;
        let max_iterations = 10;

        while iterations < max_iterations {
            iterations += 1;

            let tools = get_tool_definitions();

            match self.provider.chat(&messages, &self.model, &tools).await {
                Ok(response) => {
                    if response.tool_calls.is_empty() {
                        debug!("No tool calls, returning direct response");
                        return response.content.unwrap_or_else(|| "No response".to_string());
                    }

                    let tool_call_json: Vec<Value> = response.tool_calls.iter().map(|tc| {
                        json!({
                            "id": tc.id,
                            "type": "function",
                            "function": {
                                "name": tc.name,
                                "arguments": tc.arguments
                            }
                        })
                    }).collect();

                    messages.push(json!({
                        "role": "assistant",
                        "content": response.content.unwrap_or_default(),
                        "tool_calls": tool_call_json
                    }));

                    for tool_call in &response.tool_calls {
                        let args: HashMap<String, Value> = if tool_call.arguments.is_object() {
                            tool_call.arguments.as_object().unwrap()
                                .iter()
                                .map(|(k, v): (&String, &Value)| (k.clone(), v.clone()))
                                .collect()
                        } else {
                            HashMap::new()
                        };

                        debug!("Executing tool: {}", tool_call.name);
                        let result = execute_tool(&tool_call.name, &args, &self.workspace).await;
                        debug!("Tool result: {} bytes", result.len());

                        messages.push(json!({
                            "role": "tool",
                            "tool_call_id": tool_call.id,
                            "name": tool_call.name,
                            "content": result
                        }));
                    }
                }
                Err(e) => {
                    tracing::error!("LLM error: {}", e);
                    return format!("Error: {}", e);
                }
            }
        }

        tracing::warn!("Max iterations reached");
        "I've completed processing but reached the maximum iteration limit.".to_string()
    }

    fn system_prompt(&self) -> String {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
        format!(
            r#"You are openat, a helpful AI assistant.

Current time: {}

Your workspace at: {}

## Available Tools
You have access to tools that you can use:
- read_file: Read file contents
- write_file: Write file to disk
- list_dir: List directory contents
- exec: Execute shell commands
- web_search: Search the web for information
- web_fetch: Fetch and extract text from a URL

## Guidelines
- Use tools when needed to accomplish tasks
- Always explain what you're doing
- Write important information to files for memory"#,
            now,
            self.workspace.display()
        )
    }
}

/// Get tool definitions for the LLM
pub fn get_tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read the contents of a file at the given path.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The file path to read"
                        }
                    },
                    "required": ["path"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "Write content to a file. Creates parent directories if needed.",
                "parameters": {
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
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "list_dir",
                "description": "List the contents of a directory.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The directory path to list"
                        }
                    },
                    "required": ["path"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "exec",
                "description": "Execute a shell command and return the output.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "cmd": {
                            "type": "string",
                            "description": "The command to execute"
                        }
                    },
                    "required": ["cmd"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "web_search",
                "description": "Search the web for information. Use this when you need current events or information not in your training data.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The search query"
                        }
                    },
                    "required": ["query"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "web_fetch",
                "description": "Fetch and extract text content from a URL.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "The URL to fetch"
                        }
                    },
                    "required": ["url"]
                }
            }
        }),
    ]
}

/// Execute a tool
pub async fn execute_tool(name: &str, args: &HashMap<String, Value>, workspace: &PathBuf) -> String {
    match name {
        "read_file" => {
            if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                match fs::read_to_string(path).await {
                    Ok(content) => content,
                    Err(e) => format!("Error reading file: {}", e),
                }
            } else {
                "Error: path parameter required".to_string()
            }
        }
        "write_file" => {
            let path = args.get("path").and_then(|v| v.as_str());
            let content = args.get("content").and_then(|v| v.as_str());

            if let (Some(path), Some(content)) = (path, content) {
                if let Some(parent) = std::path::PathBuf::from(path).parent() {
                    let _ = fs::create_dir_all(parent).await;
                }
                match fs::write(path, content).await {
                    Ok(_) => format!("Successfully wrote {} bytes to {}", content.len(), path),
                    Err(e) => format!("Error writing file: {}", e),
                }
            } else {
                "Error: path and content parameters required".to_string()
            }
        }
        "list_dir" => {
            if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                match fs::read_dir(path).await {
                    Ok(mut entries) => {
                        let mut items = Vec::new();
                        while let Some(entry) = entries.next_entry().await.unwrap_or(None) {
                            items.push(entry.file_name().to_string_lossy().to_string());
                        }
                        if items.is_empty() {
                            format!("Directory {} is empty", path)
                        } else {
                            items.join("\n")
                        }
                    }
                    Err(e) => format!("Error listing directory: {}", e),
                }
            } else {
                "Error: path parameter required".to_string()
            }
        }
        "exec" => {
            if let Some(cmd) = args.get("cmd").and_then(|v| v.as_str()) {
                match tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(cmd)
                    .current_dir(workspace)
                    .output()
                    .await
                {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        format!("stdout:\n{}\nstderr:\n{}", stdout, stderr)
                    }
                    Err(e) => format!("Error executing command: {}", e),
                }
            } else {
                "Error: cmd parameter required".to_string()
            }
        }
        "web_search" => {
            if let Some(query) = args.get("query").and_then(|v| v.as_str()) {
                "Web search executed. (requires config)".to_string()
            } else {
                "Error: query parameter required".to_string()
            }
        }
        "web_fetch" => {
            if let Some(url) = args.get("url").and_then(|v| v.as_str()) {
                "Web fetch executed. (requires config)".to_string()
            } else {
                "Error: url parameter required".to_string()
            }
        }
        _ => format!("Error: Unknown tool '{}'", name),
    }
}
