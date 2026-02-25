//! Agent module for openat
//!
//! Provides agent execution with tool support and message history.

pub mod session;

use futures_util::{Stream, StreamExt};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use tokio::fs;

pub use session::{Session, SessionManager};

/// Message type for LLM
#[derive(Debug, Clone)]
pub struct Message {
    pub role: openat_types::MessageRole,
    pub content: String,
    pub name: Option<String>,
    pub tool_calls: Vec<openat_types::ToolCall>,
    pub tool_call_id: Option<String>,
}

impl Message {
    /// Create a system message
    pub fn system(content: &str) -> Self {
        Self {
            role: openat_types::MessageRole::System,
            content: content.to_string(),
            name: None,
            tool_calls: vec![],
            tool_call_id: None,
        }
    }

    /// Create a user message
    pub fn user(content: &str) -> Self {
        Self {
            role: openat_types::MessageRole::User,
            content: content.to_string(),
            name: None,
            tool_calls: vec![],
            tool_call_id: None,
        }
    }

    /// Create an assistant message
    pub fn assistant(content: &str) -> Self {
        Self {
            role: openat_types::MessageRole::Assistant,
            content: content.to_string(),
            name: None,
            tool_calls: vec![],
            tool_call_id: None,
        }
    }

    /// Convert to JSON value
    pub fn to_json(&self) -> Value {
        let mut map = serde_json::Map::new();
        map.insert("role".to_string(), json!(format!("{:?}", self.role).to_lowercase()));
        map.insert("content".to_string(), json!(self.content));

        if let Some(name) = &self.name {
            map.insert("name".to_string(), json!(name));
        }

        if !self.tool_calls.is_empty() {
            map.insert("tool_calls".to_string(), json!(self.tool_calls.iter().map(|tc| {
                json!({
                    "id": tc.id,
                    "type": "function",
                    "function": {
                        "name": tc.name,
                        "arguments": tc.arguments
                    }
                })
            }).collect::<Vec<_>>()));
        }

        if let Some(id) = &self.tool_call_id {
            map.insert("tool_call_id".to_string(), json!(id));
        }

        Value::Object(map)
    }
}

/// Agent executor that handles message processing with tools and history.
#[derive(Clone)]
pub struct AgentExecutor {
    provider: Arc<dyn openat_providers::LLMProvider>,
    session_manager: SessionManager,
    system_prompt: String,
    workspace: PathBuf,
    bus: openat_runtime::MessageBus,
    max_history_messages: usize,
    model: String,
    tool_config: openat_config::Tools,
}

/// Streaming response chunk
#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub content: String,
    pub is_final: bool,
}

impl AgentExecutor {
    /// Create a new agent executor.
    pub fn new(
        provider: Arc<dyn openat_providers::LLMProvider>,
        config: &openat_config::Config,
        bus: &openat_runtime::MessageBus,
    ) -> Self {
        let workspace = openat_config::ensure_workspace_exists();
        let sessions_dir = openat_config::workspace_path().join("sessions");

        let system_prompt = Self::build_system_prompt(&workspace);
        let model = config.agents.defaults.model.clone();

        // Store tool config for dynamic tool loading
        let tool_config = config.tools.clone();

        Self {
            provider,
            session_manager: SessionManager::new(sessions_dir),
            system_prompt,
            workspace,
            bus: bus.clone(),
            max_history_messages: 20,
            model,
            tool_config,
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
    pub async fn handle_message(
        &mut self,
        msg: &openat_types::InboundMessage,
    ) -> Result<openat_types::OutboundMessage, String> {
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
        let mut outbound = openat_types::OutboundMessage::new(&msg.channel, &msg.chat_id, &response_content);
        if let Some(msg_id) = msg.metadata.get("msg_id") {
            outbound.reply_to = Some(msg_id.clone());
        }
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
                "system" => openat_types::MessageRole::System,
                "assistant" => openat_types::MessageRole::Assistant,
                "tool" => openat_types::MessageRole::Tool,
                _ => openat_types::MessageRole::User,
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

    /// Handle an inbound message with streaming response.
    /// Returns a stream of content chunks that can be sent in real-time.
    pub fn handle_message_streaming(
        &self,
        msg: &openat_types::InboundMessage,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamChunk, String>> + Send>> {
        let msg = msg.clone();
        let system_prompt = self.system_prompt.clone();
        let provider = self.provider.clone();
        let model = self.model.clone();

        Box::pin(async_stream::stream! {
            // Check if provider supports streaming
            if !provider.supports_streaming() {
                yield Err("Provider does not support streaming".to_string());
                return;
            }

            // Build messages
            let mut messages = vec![Message::system(&system_prompt)];
            messages.push(Message::user(&msg.content));

            let messages_json: Vec<Value> = messages.iter().map(|m| m.to_json()).collect();

            // Get stream from provider
            let tool_defs: Vec<Value> = vec![];
            let mut stream = provider.stream(&messages_json, &model, &tool_defs);

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        yield Ok(StreamChunk {
                            content: chunk.content,
                            is_final: chunk.is_final,
                        });
                    }
                    Err(e) => {
                        yield Err(e);
                        return;
                    }
                }
            }
        })
    }

    /// Get tool definitions for the LLM based on config.
    fn get_tool_definitions(&self) -> Vec<openat_types::ToolDefinition> {
        let mut tools = Vec::new();
        let config = &self.tool_config;

        // Filesystem tools (read_file, write_file, list_dir)
        if config.filesystem {
            tools.push(openat_types::ToolDefinition::new(
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
            ));
            tools.push(openat_types::ToolDefinition::new(
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
            ));
            tools.push(openat_types::ToolDefinition::new(
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
            ));
        }

        // Shell tool
        if config.shell {
            tools.push(openat_types::ToolDefinition::new(
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
            ));
        }

        // Web search (requires enabled + api_key)
        if config.web_search.enabled && !config.web_search.api_key.is_empty() {
            tools.push(openat_types::ToolDefinition::new(
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
            ));
        }

        // Web fetch
        if config.web_fetch {
            tools.push(openat_types::ToolDefinition::new(
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
            ));
        }

        tools
    }

    /// Chat with tool support.
    async fn chat_with_tools(
        &mut self,
        messages: &[Message],
        tools: &[openat_types::ToolDefinition],
    ) -> Result<openat_providers::LLMResponse, String> {
        let mut messages_json: Vec<Value> = messages.iter().map(|m| m.to_json()).collect();
        let tool_defs_json: Vec<Value> = tools.iter().map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters
                }
            })
        }).collect();

        let mut iterations = 0;
        let max_iterations = 30;

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
                        tracing::info!("Executing tool: {} with args: {}", tool_call.name, tool_call.arguments);
                        let result = self.execute_tool(&tool_call.name, &tool_call.arguments).await;
                        tracing::info!("Tool result: {}", result);
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
        self.model.clone()
    }

    /// Execute a tool.
    async fn execute_tool(&self, name: &str, arguments: &Value) -> String {
        // Check if tool is enabled in config
        let config = &self.tool_config;
        match name {
            "read_file" | "write_file" | "list_dir" => {
                if !config.filesystem {
                    return "Error: filesystem tool is disabled".to_string();
                }
            }
            "exec" => {
                if !config.shell {
                    return "Error: shell tool is disabled".to_string();
                }
            }
            "web_search" => {
                if !config.web_search.enabled || config.web_search.api_key.is_empty() {
                    return "Error: web_search tool is disabled".to_string();
                }
            }
            "web_fetch" => {
                if !config.web_fetch {
                    return "Error: web_fetch tool is disabled".to_string();
                }
            }
            _ => {}
        }

        // Handle arguments that are wrapped in a string (common with some LLMs)
        let args: HashMap<String, Value> = if arguments.is_string() {
            // Try to parse as JSON string
            let arg_str = arguments.as_str().unwrap_or("");
            if arg_str.starts_with("{") {
                serde_json::from_str(arg_str).unwrap_or_else(|_| HashMap::new())
            } else {
                HashMap::new()
            }
        } else if arguments.is_object() {
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
                    match validate_path(path, &self.workspace, self.tool_config.restrict_to_workspace) {
                        Ok(expanded_path) => {
                            match fs::read_to_string(&expanded_path).await {
                                Ok(content) => content,
                                Err(e) => format!("Error reading file {}: {}", expanded_path, e),
                            }
                        }
                        Err(e) => e,
                    }
                } else {
                    "Error: path parameter required".to_string()
                }
            }
            "write_file" => {
                let path = args.get("path").and_then(|v| v.as_str());
                let content = args.get("content").and_then(|v| v.as_str());

                if let (Some(path), Some(content)) = (path, content) {
                    match validate_path(path, &self.workspace, self.tool_config.restrict_to_workspace) {
                        Ok(expanded_path) => {
                            if let Some(parent) = std::path::PathBuf::from(&expanded_path).parent() {
                                let _ = fs::create_dir_all(parent).await;
                            }
                            match fs::write(&expanded_path, content).await {
                                Ok(_) => format!("Successfully wrote {} bytes to {}", content.len(), expanded_path),
                                Err(e) => format!("Error writing file {}: {}", expanded_path, e),
                            }
                        }
                        Err(e) => e,
                    }
                } else {
                    "Error: path and content parameters required".to_string()
                }
            }
            "list_dir" => {
                if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                    match validate_path(path, &self.workspace, self.tool_config.restrict_to_workspace) {
                        Ok(expanded_path) => {
                            match fs::read_dir(&expanded_path).await {
                                Ok(mut entries) => {
                                    let mut items = Vec::new();
                                    while let Some(entry) = entries.next_entry().await.unwrap_or(None) {
                                        items.push(entry.file_name().to_string_lossy().to_string());
                                    }
                                    if items.is_empty() {
                                        format!("Directory {} is empty", expanded_path)
                                    } else {
                                        items.join("\n")
                                    }
                                }
                                Err(e) => format!("Error listing directory {}: {}", expanded_path, e),
                            }
                        }
                        Err(e) => e,
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
                            if !stderr.is_empty() {
                                format!("stdout:\n{}\nstderr:\n{}", stdout, stderr)
                            } else {
                                stdout.to_string()
                            }
                        }
                        Err(e) => format!("Error executing command: {}", e),
                    }
                } else {
                    "Error: cmd parameter required".to_string()
                }
            }
            "web_search" => {
                if let Some(query) = args.get("query").and_then(|v| v.as_str()) {
                    format!("Web search for '{}' would be executed here.", query)
                } else {
                    "Error: query parameter required".to_string()
                }
            }
            "web_fetch" => {
                if let Some(url) = args.get("url").and_then(|v| v.as_str()) {
                    format!("Web fetch for '{}' would be executed here.", url)
                } else {
                    "Error: url parameter required".to_string()
                }
            }
            _ => format!("Error: Unknown tool '{}'", name),
        }
    }
}

/// Expand ~ to home directory
fn expand_path(path: &str) -> String {
    if path.starts_with("~") {
        match std::env::var("HOME") {
            Ok(home) => path.replace("~", &home),
            Err(_) => path.to_string(),
        }
    } else {
        path.to_string()
    }
}

/// Validate that a path is within the allowed workspace
fn validate_path(path: &str, workspace: &std::path::Path, restrict: bool) -> Result<String, String> {
    let expanded = expand_path(path);
    let path_buf = std::path::PathBuf::from(&expanded);

    if restrict {
        // Resolve both paths to canonical form
        let ws_canonical = workspace.canonicalize()
            .unwrap_or_else(|_| workspace.to_path_buf());
        let path_canonical = path_buf.canonicalize()
            .unwrap_or_else(|_| path_buf.clone());

        // Check if path is within workspace
        if !path_canonical.starts_with(&ws_canonical) {
            return Err(format!(
                "Access denied: path '{}' is outside workspace '{}'",
                expanded,
                workspace.display()
            ));
        }
    }

    Ok(expanded)
}
