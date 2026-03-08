//! Agent module for openat
//!
//! Provides agent execution with tool support and message history.

pub mod session;
pub mod subagent;
pub mod cron;
pub mod memory;
mod util;

use futures_util::{Stream, StreamExt};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use tokio::fs;

pub use session::{Session, SessionManager};
pub use openat_tools::skill::SkillManager;
pub use subagent::SubagentManager;
pub use cron::{CronService, CronJob, CronSchedule, CronPayload, ScheduleKind};

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
pub struct AgentExecutor {
    provider: Arc<dyn openat_providers::LLMProvider>,
    session_manager: SessionManager,
    skill_manager: SkillManager,
    subagent_manager: SubagentManager,
    cron_service: Option<CronService>,
    memory_store: Option<crate::memory::MemoryStore>,
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

        // Initialize skill manager with default skills
        let skill_manager = SkillManager::new();
        // Note: Skills will be initialized lazily on first use

        // Initialize subagent manager
        let subagent_manager = SubagentManager::new(
            provider.clone(),
            workspace.clone(),
            model.clone(),
            bus.clone(),
            config.tools.subagent.max_iterations,
            config.tools.subagent.enabled,
        );

        // Initialize cron service (optional, based on config)
        let cron_service = if config.tools.cron.enabled {
            let cron_store_path = openat_config::workspace_path().join("cron").join("jobs.json");
            let service = CronService::new(cron_store_path, bus.clone());
            Some(service)
        } else {
            None
        };

        // Initialize memory store (optional, based on config)
        let memory_store = if config.tools.memory.enabled {
            let store = crate::memory::MemoryStore::new(workspace.clone());
            Some(store)
        } else {
            None
        };

        Self {
            provider,
            session_manager: SessionManager::new(sessions_dir.clone()),
            skill_manager: skill_manager.clone(),
            subagent_manager,
            cron_service,
            memory_store,
            system_prompt,
            workspace: sessions_dir.parent().unwrap().to_path_buf(),
            bus: bus.clone(),
            max_history_messages: 20,
            model,
            tool_config,
        }
    }

    /// Load skills from workspace on initialization
    pub async fn init_skills(&self) {
        // Initialize default skills
        self.skill_manager.init_default_skills().await;

        // Load skills from workspace
        let workspace = openat_config::workspace_path();
        self.skill_manager.load_from_workspace(&workspace).await;
    }

    /// Initialize and start cron service
    pub async fn init_cron(&self) {
        if let Some(ref cron_service) = self.cron_service {
            if let Err(e) = cron_service.load().await {
                tracing::error!("Failed to load cron jobs: {}", e);
            }
            cron_service.start().await;
            tracing::info!("Cron service started");
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

        // Check for matching skills
        let skill_prompt = self.find_matching_skill(&msg.content).await;

        // Add user message to history
        session.add_message(util::role::USER, &msg.content);

        // Build message history for LLM
        let mut messages = self.build_message_history(&session);

        // Prepend skill prompt if matched
        if let Some(prompt) = skill_prompt {
            if let Some(first_msg) = messages.first_mut() {
                first_msg.content = format!("{}\n\n{}", prompt, first_msg.content);
            }
        }

        // Get tool definitions
        let tools = self.get_tool_definitions();

        // Execute chat with tool support
        let response = self.chat_with_tools(&messages, &tools).await?;

        // Add assistant response to history
        let response_content = response.content.clone().unwrap_or_default();
        session.add_message(util::role::ASSISTANT, &response_content);

        // Save session (async, fire and forget)
        let sm = self.session_manager.clone();
        tokio::spawn(async move {
            sm.save(&session).await;
        });

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

    /// Find a matching skill for the given message content
    async fn find_matching_skill(&self, content: &str) -> Option<String> {
        let skills = self.skill_manager.find_by_trigger(content).await;

        if let Some(skill) = skills.first() {
            tracing::info!("Skill triggered: {} (id: {})", skill.name, skill.id);
            tracing::debug!("Skill prompt: {}", skill.prompt);
            return Some(skill.prompt.clone());
        }

        tracing::debug!("No skill matched for content: {}", &content[..content.len().min(50)]);
        None
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

        // Subagent tool - spawn background subagent for complex tasks (if enabled in config)
        if config.subagent.enabled {
            tools.push(openat_types::ToolDefinition::new(
                "spawn_subagent",
                "Spawn a subagent to execute a task in the background. Use this for complex tasks that can run independently.",
                json!({
                    "type": "object",
                    "properties": {
                        "task": {
                            "type": "string",
                            "description": "The task description for the subagent"
                        },
                        "label": {
                            "type": "string",
                            "description": "Optional label to identify this subagent task"
                        }
                    },
                    "required": ["task"]
                }),
            ));
            tools.push(openat_types::ToolDefinition::new(
                "list_subagents",
                "List all running subagents and their status.",
                json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            ));
        }

        // Cron tool - schedule reminders and recurring tasks (if enabled in config)
        if config.cron.enabled {
            tools.push(openat_types::ToolDefinition::new(
                "cron",
                "Schedule reminders and recurring tasks. Actions: add, list, remove.",
                json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["add", "list", "remove"],
                            "description": "Action to perform"
                        },
                        "message": {
                            "type": "string",
                            "description": "Reminder message (for add)"
                        },
                        "every_seconds": {
                            "type": "integer",
                            "description": "Interval in seconds (for recurring tasks)"
                        },
                        "cron_expr": {
                            "type": "string",
                            "description": "Cron expression like '0 9 * * *' (for scheduled tasks)"
                        },
                        "job_id": {
                            "type": "string",
                            "description": "Job ID (for remove)"
                        }
                    },
                    "required": ["action"]
                }),
            ));
        }

        // Memory tools - save and recall memory (if enabled in config)
        if config.memory.enabled {
            tools.push(openat_types::ToolDefinition::new(
                "save_memory",
                "Save important information to long-term memory for future reference.",
                json!({
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "The important information to remember"
                        }
                    },
                    "required": ["content"]
                }),
            ));
            tools.push(openat_types::ToolDefinition::new(
                "recall_memory",
                "Search and recall information from long-term memory.",
                json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query to find relevant memories"
                        }
                    },
                    "required": ["query"]
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
        let args: HashMap<String, Value> = util::parse_tool_arguments(arguments);

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
            "spawn_subagent" => {
                let task = args.get("task").and_then(|v| v.as_str());
                let label = args.get("label").and_then(|v| v.as_str());

                if let Some(task) = task {
                    let label_opt = label.map(String::from);
                    self.subagent_manager
                        .spawn(
                            task.to_string(),
                            label_opt,
                            util::channel::CLI.to_string(),
                            util::target::DIRECT.to_string(),
                            None,
                        )
                        .await
                } else {
                    "Error: task parameter required".to_string()
                }
            }
            "list_subagents" => {
                let count = self.subagent_manager.get_running_count().await;
                format!("Currently running subagents: {}", count)
            }
            "cron" => {
                if let Some(ref cron_service) = self.cron_service {
                    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");

                    match action {
                        "add" => {
                            let message = args.get("message").and_then(|v| v.as_str()).unwrap_or("");
                            let every_seconds = args.get("every_seconds").and_then(|v| v.as_i64());
                            let cron_expr = args.get("cron_expr").and_then(|v| v.as_str());

                            if message.is_empty() {
                                return "Error: message is required for add".to_string();
                            }

                            let (_schedule_kind, schedule) = if let Some(every) = every_seconds {
                                let schedule = CronSchedule {
                                    kind: ScheduleKind::Every,
                                    at_ms: None,
                                    every_ms: Some(every * 1000),
                                    expr: None,
                                    tz: None,
                                };
                                (ScheduleKind::Every, schedule)
                            } else if let Some(expr) = cron_expr {
                                let schedule = CronSchedule {
                                    kind: ScheduleKind::Cron,
                                    at_ms: None,
                                    every_ms: None,
                                    expr: Some(expr.to_string()),
                                    tz: None,
                                };
                                (ScheduleKind::Cron, schedule)
                            } else {
                                return "Error: either every_seconds or cron_expr is required".to_string();
                            };

                            let payload = CronPayload {
                                message: message.to_string(),
                                channel: Some(util::channel::CLI.to_string()),
                                to: Some(util::target::DIRECT.to_string()),
                            };

                            let job_id = cron_service.add_job("Reminder".to_string(), schedule, payload).await;
                            format!("Cron job added with ID: {}", job_id)
                        }
                        "list" => {
                            let jobs = cron_service.list_jobs().await;
                            if jobs.is_empty() {
                                "No cron jobs scheduled".to_string()
                            } else {
                                let mut output = String::from("Scheduled cron jobs:\n");
                                for job in &jobs {
                                    let schedule_str = match job.schedule.kind {
                                        ScheduleKind::At => format!("at {}", job.schedule.at_ms.unwrap_or(0)),
                                        ScheduleKind::Every => format!("every {}s", job.schedule.every_ms.unwrap_or(0) / 1000),
                                        ScheduleKind::Cron => job.schedule.expr.clone().unwrap_or_default(),
                                    };
                                    output.push_str(&format!(
                                        "- {}: {} [{}] ({})\n",
                                        job.id,
                                        job.payload.message,
                                        schedule_str,
                                        if job.enabled { "enabled" } else { "disabled" }
                                    ));
                                }
                                output
                            }
                        }
                        "remove" => {
                            let job_id = args.get("job_id").and_then(|v| v.as_str()).unwrap_or("");
                            if job_id.is_empty() {
                                return "Error: job_id is required for remove".to_string();
                            }
                            if cron_service.remove_job(job_id).await {
                                format!("Cron job {} removed", job_id)
                            } else {
                                format!("Cron job {} not found", job_id)
                            }
                        }
                        _ => format!("Unknown action: {}. Use add, list, or remove.", action),
                    }
                } else {
                    "Error: Cron service is not enabled".to_string()
                }
            }
            "save_memory" => {
                if let Some(ref memory_store) = self.memory_store {
                    let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
                    if content.is_empty() {
                        return "Error: content parameter required".to_string();
                    }

                    // Append to memory
                    let current = match memory_store.read_long_term().await {
                        Ok(c) => c,
                        Err(_) => String::new(),
                    };

                    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
                    let new_memory = if current.is_empty() {
                        format!("- [{}] {}", timestamp, content)
                    } else {
                        format!("{}\n- [{}] {}", current, timestamp, content)
                    };

                    match memory_store.write_long_term(&new_memory).await {
                        Ok(_) => "Memory saved successfully".to_string(),
                        Err(e) => format!("Error saving memory: {}", e),
                    }
                } else {
                    "Error: Memory service is not enabled".to_string()
                }
            }
            "recall_memory" => {
                if let Some(ref memory_store) = self.memory_store {
                    let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                    if query.is_empty() {
                        return "Error: query parameter required".to_string();
                    }

                    // Read memory and do simple search
                    match memory_store.read_long_term().await {
                        Ok(memory) if !memory.is_empty() => {
                            // Simple case-insensitive search
                            let query_lower = query.to_lowercase();
                            let lines: Vec<&str> = memory.lines()
                                .filter(|line| line.to_lowercase().contains(&query_lower))
                                .collect();

                            if lines.is_empty() {
                                format!("No memories found matching '{}'", query)
                            } else {
                                let results: Vec<&str> = lines.iter().take(5).copied().collect();
                                format!("Relevant memories:\n{}", results.join("\n"))
                            }
                        }
                        _ => format!("No memories found matching '{}'", query),
                    }
                } else {
                    "Error: Memory service is not enabled".to_string()
                }
            }
            _ => format!("Error: Unknown tool '{}'", name),
        }
    }
}

/// Validate that a path is within the allowed workspace
fn validate_path(path: &str, workspace: &std::path::Path, restrict: bool) -> Result<String, String> {
    let expanded = util::expand_path(path);
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
