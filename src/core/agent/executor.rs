//! Agent Executor - Core agent logic with tool support and message history.

use crate::config::Config;
use crate::core::bus::MessageBus;
use crate::core::session::{Session, SessionManager};
use crate::llm::LLMProvider;
use crate::types::{InboundMessage, LLMResponse, Message, OutboundMessage, ToolCall, ToolDefinition, ToolResult};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

/// Agent executor that handles message processing with tools and history.
pub struct AgentExecutor {
    provider: Box<dyn LLMProvider>,
    session_manager: SessionManager,
    system_prompt: String,
    workspace: PathBuf,
    bus: MessageBus,
    max_history_messages: usize,
}

impl AgentExecutor {
    /// Create a new agent executor.
    pub fn new(provider: Box<dyn LLMProvider>, config: &Config, bus: &MessageBus) -> Self {
        let workspace = crate::config::ensure_workspace_exists();
        let sessions_dir = crate::config::workspace_path().join("sessions");

        let system_prompt = Self::build_system_prompt(&workspace);

        Self {
            provider,
            session_manager: SessionManager::new(sessions_dir),
            system_prompt,
            workspace,
            bus: bus.clone(),
            max_history_messages: 20,
        }
    }

    /// Build the system prompt for the agent.
    fn build_system_prompt(workspace: &PathBuf) -> String {
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
            workspace.display()
        )
    }

    /// Handle an inbound message and produce an outbound response.
    pub async fn handle_message(&mut self, msg: &InboundMessage) -> Result<OutboundMessage, String> {
        let session_key = msg.session_key();

        // Load or create session
        let mut session = self.session_manager.load(&session_key).unwrap_or_else(|| {
            Session::new(session_key)
        });

        // Add user message to history
        session.add_message("user", &msg.content);

        // Build message history for LLM
        let messages = self.build_message_history(&session);

        // Get tool definitions
        let tools = self.get_tool_definitions();

        // Execute chat with tool support
        let response = self.chat_with_tools(&messages, &tools).await?;

        // Add assistant response to history
        let response_content = response.content.clone().unwrap_or_default();
        session.add_message("assistant", &response_content);

        // Save session
        self.session_manager.save(&session);

        // Publish response to bus
        let outbound = OutboundMessage::new(&msg.channel, &msg.chat_id, &response_content);
        self.bus.publish_outbound(outbound.clone()).await;

        Ok(outbound)
    }

    /// Build message history for the LLM.
    fn build_message_history(&self, session: &Session) -> Vec<Message> {
        let mut messages = Vec::new();

        // Add system prompt
        messages.push(Message::system(&self.system_prompt));

        // Get recent history
        let history = session.get_history(self.max_history_messages);

        // Convert history to Message structs
        for msg in history {
            let role = match msg.get("role").map(|s| s.as_str()).unwrap_or("user") {
                "system" => crate::types::MessageRole::System,
                "assistant" => crate::types::MessageRole::Assistant,
                "tool" => crate::types::MessageRole::Tool,
                _ => crate::types::MessageRole::User,
            };

            let content = msg.get("content").map(|s| s.to_string()).unwrap_or_default();

            let message = Message {
                role,
                content,
                name: None,
                tool_calls: vec![],
                tool_call_id: None,
            };
            messages.push(message);
        }

        messages
    }

    /// Get tool definitions for the LLM.
    fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        vec![
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
            ),
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
            ),
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
            ),
            ToolDefinition::new(
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
            ToolDefinition::new(
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
            ToolDefinition::new(
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

    /// Chat with tool support.
    async fn chat_with_tools(
        &mut self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<LLMResponse, String> {
        let mut messages_json: Vec<Value> = messages.iter().map(|m| m.to_json()).collect();
        let tool_defs_json: Vec<Value> = tools.iter().map(|t| t.to_json()).collect();

        let mut iterations = 0;
        let max_iterations = 10;

        while iterations < max_iterations {
            iterations += 1;

            match self
                .provider
                .chat(&messages_json, &self.get_model(), &tool_defs_json)
                .await
            {
                Ok(response) => {
                    if response.tool_calls.is_empty() {
                        return Ok(response);
                    }

                    // Add assistant message with tool calls
                    let content = response.content.clone().unwrap_or_default();
                    messages_json.push(json!({
                        "role": "assistant",
                        "content": content,
                        "tool_calls": response.tool_calls.iter().map(|tc| {
                            json!({
                                "id": tc.id,
                                "type": "function",
                                "function": {
                                    "name": tc.name,
                                    "arguments": tc.arguments
                                }
                            })
                        }).collect::<Vec<_>>()
                    }));

                    // Execute tools
                    for tool_call in &response.tool_calls {
                        let result = self.execute_tool(&tool_call.name, &tool_call.arguments).await;
                        messages_json.push(json!({
                            "role": "tool",
                            "tool_call_id": tool_call.id,
                            "name": tool_call.name,
                            "content": result
                        }));
                    }
                }
                Err(e) => return Err(e),
            }
        }

        Err("Maximum iteration limit reached".to_string())
    }

    /// Get the model name from config.
    fn get_model(&self) -> String {
        // Default model - could be extended to read from config
        "anthropic/claude-opus-4-5".to_string()
    }

    /// Execute a tool.
    async fn execute_tool(&self, name: &str, arguments: &Value) -> String {
        let args = if arguments.is_object() {
            arguments
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        } else {
            HashMap::new()
        };

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
                        .current_dir(&self.workspace)
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
                    // Web search would use the Brave Search API
                    format!("Web search for '{}' would be executed here.", query)
                } else {
                    "Error: query parameter required".to_string()
                }
            }
            "web_fetch" => {
                if let Some(url) = args.get("url").and_then(|v| v.as_str()) {
                    // Web fetch would fetch the URL content
                    format!("Web fetch for '{}' would be executed here.", url)
                } else {
                    "Error: url parameter required".to_string()
                }
            }
            _ => format!("Error: Unknown tool '{}'", name),
        }
    }
}

/// Helper function to convert ToolCall to ToolResult
impl From<ToolCall> for ToolResult {
    fn from(tc: ToolCall) -> Self {
        ToolResult {
            name: tc.name,
            success: true,
            content: String::new(),
            error: None,
        }
    }
}
