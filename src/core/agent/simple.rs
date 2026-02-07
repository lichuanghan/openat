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
                    debug!("LLM response: content={:?}, tool_calls={}",
                        response.content.as_ref().map(|s| s.len()),
                        response.tool_calls.len());

                    // If no tool calls, return the response directly
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

                    // Execute all tool calls
                    let mut tool_results = Vec::new();
                    for tool_call in &response.tool_calls {
                        // MiniMax may return arguments as a string instead of JSON object
                        let args: HashMap<String, Value> = if tool_call.arguments.is_object() {
                            tool_call.arguments.as_object().unwrap()
                                .iter()
                                .map(|(k, v): (&String, &Value)| (k.clone(), v.clone()))
                                .collect()
                        } else if tool_call.arguments.is_string() {
                            // Parse string arguments as JSON
                            let args_str = tool_call.arguments.as_str().unwrap_or("{}");
                            if let Ok(obj) = serde_json::from_str::<Value>(args_str) {
                                if obj.is_object() {
                                    obj.as_object().unwrap()
                                        .iter()
                                        .map(|(k, v)| (k.clone(), v.clone()))
                                        .collect()
                                } else {
                                    HashMap::new()
                                }
                            } else {
                                HashMap::new()
                            }
                        } else {
                            HashMap::new()
                        };

                        debug!("Executing tool: {} with args: {:?}", tool_call.name, args);
                        let result = execute_tool(&tool_call.name, &args, &self.workspace).await;
                        debug!("Tool result: {} bytes", result.len());

                        tool_results.push(json!({
                            "role": "tool",
                            "tool_call_id": tool_call.id,
                            "name": tool_call.name,
                            "content": result
                        }));
                    }

                    // Add all tool results to messages
                    for result in &tool_results {
                        messages.push(result.clone());
                    }

                    // For MiniMax and similar models that keep calling tools,
                    // we need to explicitly ask for a final response without more tool calls
                    messages.push(json!({
                        "role": "user",
                        "content": "重要提示：工具已经执行完成，上面的 tool 消息就是执行结果。请基于这个结果直接给出最终回答，绝对不要再调用任何工具。"
                    }));
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
            r#"You are OpenAt, a helpful AI assistant.

Current time: {}
Workspace: {}

## Tool Usage Rules
When you request to use a tool, the system will automatically execute it and return the result to you. You will see messages with role: "tool" containing the execution results.

IMPORTANT - After receiving tool results:
1. You MUST provide a final answer based on the tool results
2. You MUST NOT request any more tools
3. NEVER say "parameter required" - tools are already executed
4. Simply report the tool result and give your answer

## Available Tools
- read_file: Read file contents (params: path)
- write_file: Write content to file, creates directories as needed (params: path, content)
- list_dir: List directory contents (params: path)
- exec: Execute shell command and return output (params: cmd)
- web_search: Search the web for information (params: query)
- web_fetch: Fetch and extract text from URL (params: url)

## Guidelines
- Use tools to complete user requests
- After tool execution, report results and give your answer
- Do not request additional tools after receiving results"#,
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
                "description": "Read the contents of a file at the specified path",
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
                "description": "Write content to a file, creates parent directories if needed",
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
                "description": "List the contents of a directory",
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
                "description": "Execute a shell command and return the output",
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
                "description": "在网上搜索信息，当需要当前事件或训练数据中没有的信息时使用",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "搜索查询"
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
                "description": "从 URL 获取并提取文本内容",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "要获取的 URL"
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
                            format!("Directory contents:\n{}", items.join("\n"))
                        }
                    }
                    Err(e) => format!("Error reading directory: {}", e),
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
                        if !stdout.is_empty() {
                            format!("Command output:\n{}", stdout.trim())
                        } else if !stderr.is_empty() {
                            format!("Error output:\n{}", stderr.trim())
                        } else {
                            "Command executed successfully, no output".to_string()
                        }
                    }
                    Err(e) => format!("Command execution failed: {}", e),
                }
            } else {
                "Error: cmd parameter required".to_string()
            }
        }
        "web_search" => {
            if let Some(_query) = args.get("query").and_then(|v| v.as_str()) {
                "Web search executed. (requires config)".to_string()
            } else {
                "Error: query parameter required".to_string()
            }
        }
        "web_fetch" => {
            if let Some(_url) = args.get("url").and_then(|v| v.as_str()) {
                "Web fetch executed. (requires config)".to_string()
            } else {
                "Error: url parameter required".to_string()
            }
        }
        _ => format!("Error: Unknown tool '{}'", name),
    }
}
