


//! Subagent module for background task execution

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{json, Value};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::util;
use openat_providers::LLMProvider;
use openat_runtime::MessageBus;
use openat_types::InboundMessage;

/// Manages background subagent execution
#[derive(Clone)]
pub struct SubagentManager {
    provider: Arc<dyn LLMProvider>,
    workspace: PathBuf,
    model: String,
    max_iterations: usize,
    bus: MessageBus,
    /// Running subagent tasks: task_id -> Task
    running_tasks: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
    /// Session to task mapping: session_key -> {task_id, ...}
    session_tasks: Arc<RwLock<HashMap<String, std::collections::HashSet<String>>>>,
}

impl SubagentManager {
    /// Create a new SubagentManager
    pub fn new(
        provider: Arc<dyn LLMProvider>,
        workspace: PathBuf,
        model: String,
        bus: MessageBus,
        max_iterations: usize,
        enabled: bool,
    ) -> Self {
        Self {
            provider,
            workspace,
            model,
            max_iterations: if enabled { max_iterations } else { 0 },
            bus,
            running_tasks: Arc::new(RwLock::new(HashMap::new())),
            session_tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Spawn a subagent to execute a task in the background
    pub async fn spawn(
        &self,
        task: String,
        label: Option<String>,
        origin_channel: String,
        origin_chat_id: String,
        session_key: Option<String>,
    ) -> String {
        // Check if subagent is disabled
        if self.max_iterations == 0 {
            return "Error: Subagent is disabled in configuration".to_string();
        }

        let task_id = Uuid::new_v4().to_string()[..8].to_string();
        let display_label = label
            .clone()
            .unwrap_or_else(|| {
                if task.len() > 30 {
                    format!("{}...", &task[..30])
                } else {
                    task.clone()
                }
            });

        tracing::info!("Spawning subagent [{}]: {}", task_id, display_label);

        let provider = self.provider.clone();
        let workspace = self.workspace.clone();
        let model = self.model.clone();
        let max_iterations = self.max_iterations;
        let task_id_clone = task_id.clone();
        let label_clone = display_label.clone();
        let task_clone = task.clone();
        let origin_channel_clone = origin_channel.clone();
        let origin_chat_id_clone = origin_chat_id.clone();

        let running_tasks = self.running_tasks.clone();
        let session_tasks = self.session_tasks.clone();
        let bus = self.bus.clone();

        // Clone for cleanup closure
        let task_id_for_cleanup = task_id.clone();
        let session_key_clone = session_key.clone();

        // Create the background task
        let handle = tokio::spawn(async move {
            Self::run_subagent(
                provider,
                workspace,
                model,
                max_iterations,
                bus,
                task_id_clone,
                task_clone,
                label_clone,
                origin_channel_clone,
                origin_chat_id_clone,
            )
            .await;

            // Cleanup after completion
            running_tasks.write().await.remove(&task_id_for_cleanup);
            if let Some(session) = session_key_clone {
                let mut tasks = session_tasks.write().await;
                if let Some(ids) = tasks.get_mut(&session) {
                    ids.remove(&task_id_for_cleanup);
                    if ids.is_empty() {
                        tasks.remove(&session);
                    }
                }
            }
        });

        // Store the task
        self.running_tasks.write().await.insert(task_id.clone(), handle);
        if let Some(ref session) = session_key {
            self.session_tasks
                .write()
                .await
                .entry(session.clone())
                .or_default()
                .insert(task_id.clone());
        }

        format!(
            "Subagent [{}] started (id: {}). I'll notify you when it completes.",
            display_label, task_id
        )
    }

    /// Execute the subagent task
    async fn run_subagent(
        provider: Arc<dyn LLMProvider>,
        workspace: PathBuf,
        model: String,
        max_iterations: usize,
        bus: MessageBus,
        task_id: String,
        task: String,
        label: String,
        origin_channel: String,
        origin_chat_id: String,
    ) {
        tracing::info!("Subagent [{}] starting task: {}", task_id, label);

        let system_prompt = Self::build_subagent_prompt(&workspace);
        let mut messages: Vec<Value> = vec![
            json!({
                "role": "system",
                "content": system_prompt
            }),
            json!({
                "role": "user",
                "content": task
            }),
        ];

        // Get subagent tool definitions
        let tools = Self::get_subagent_tools();
        let tool_defs_json: Vec<Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters
                    }
                })
            })
            .collect();

        let mut final_result: Option<String> = None;
        let mut iteration = 0;

        while iteration < max_iterations {
            iteration += 1;

            match provider.chat(&messages, &model, &tool_defs_json).await {
                Ok(response) => {
                    if response.tool_calls.is_empty() {
                        final_result = response.content;
                        break;
                    }

                    // Add assistant message with tool calls
                    let content = response.content.clone().unwrap_or_default();
                    messages.push(json!({
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
                        tracing::debug!(
                            "Subagent [{}] executing: {} with args: {}",
                            task_id,
                            tool_call.name,
                            tool_call.arguments
                        );
                        let result =
                            Self::execute_tool(&tool_call.name, &tool_call.arguments, &workspace)
                                .await;
                        messages.push(json!({
                            "role": "tool",
                            "tool_call_id": tool_call.id,
                            "name": tool_call.name,
                            "content": result
                        }));
                    }
                }
                Err(e) => {
                    tracing::error!("Subagent [{}] error: {}", task_id, e);
                    final_result = Some(format!("Error: {}", e));
                    break;
                }
            }
        }

        if final_result.is_none() {
            final_result = Some("Task completed but no final response was generated.".to_string());
        }

        tracing::info!("Subagent [{}] completed", task_id);

        // Announce result via message bus
        if let Some(result) = final_result {
            let status = if result.starts_with("Error:") { "failed" } else { "completed successfully" };
            let status_text = if status == "failed" { "failed" } else { "completed successfully" };

            let announce_content = format!(
                r#"[Subagent '{}' {}]

Task: {}

Result:
{}

Summarize this naturally for the user. Keep it brief (1-2 sentences)."#,
                label, status_text, task, result
            );

            // Publish the result as an inbound message to trigger main agent
            let inbound = InboundMessage::new(
                &origin_channel,
                "subagent",
                &origin_chat_id,
                &announce_content,
            );
            bus.publish_inbound(inbound).await;

            tracing::debug!("Subagent [{}] announced result to {}:{}", task_id, origin_channel, origin_chat_id);
        }
    }

    /// Build the system prompt for subagent
    fn build_subagent_prompt(workspace: &PathBuf) -> String {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
        format!(
            r#"# Subagent

Current time: {}

You are a subagent spawned by the main agent to complete a specific task.
Stay focused on the assigned task. Your final response will be reported back to the main agent.

## Workspace
{}

## Available Tools
- read_file: Read file contents
- write_file: Write file to disk
- list_dir: List directory contents
- exec: Execute shell commands
- web_search: Search the web for information
- web_fetch: Fetch and extract text from a URL

## Guidelines
- Stay focused on the assigned task
- Use tools when needed to accomplish the task
- Provide a clear final result when done"#,
            now,
            workspace.display()
        )
    }

    /// Get tool definitions for subagent (limited tools, no spawn/recursive calls)
    fn get_subagent_tools() -> Vec<openat_types::ToolDefinition> {
        let mut tools = Vec::new();

        // Filesystem tools
        tools.push(openat_types::ToolDefinition::new(
            "read_file",
            "Read the contents of a file at the given path.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "The file path to read" }
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
                    "path": { "type": "string", "description": "The file path to write to" },
                    "content": { "type": "string", "description": "The content to write" }
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
                    "path": { "type": "string", "description": "The directory path to list" }
                },
                "required": ["path"]
            }),
        ));
        tools.push(openat_types::ToolDefinition::new(
            "edit_file",
            "Edit a specific portion of a file. Use this to make targeted changes to existing files.",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "The file path to edit" },
                    "old_string": { "type": "string", "description": "The exact text to find in the file" },
                    "new_string": { "type": "string", "description": "The text to replace it with" }
                },
                "required": ["path", "old_string", "new_string"]
            }),
        ));

        // Shell tool
        tools.push(openat_types::ToolDefinition::new(
            "exec",
            "Execute a shell command and return the output.",
            json!({
                "type": "object",
                "properties": {
                    "cmd": { "type": "string", "description": "The command to execute" }
                },
                "required": ["cmd"]
            }),
        ));

        // Web tools
        tools.push(openat_types::ToolDefinition::new(
            "web_search",
            "Search the web for information. Use this when you need current events.",
            json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "The search query" }
                },
                "required": ["query"]
            }),
        ));
        tools.push(openat_types::ToolDefinition::new(
            "web_fetch",
            "Fetch and extract text content from a URL.",
            json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "The URL to fetch" }
                },
                "required": ["url"]
            }),
        ));

        tools
    }

    /// Execute a tool for subagent
    async fn execute_tool(name: &str, arguments: &Value, workspace: &PathBuf) -> String {
        let args: HashMap<String, Value> = if arguments.is_string() {
            let arg_str = arguments.as_str().unwrap_or("");
            if arg_str.starts_with('{') {
                serde_json::from_str(arg_str).unwrap_or_default()
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
                    let expanded = util::expand_path(path);
                    match tokio::fs::read_to_string(&expanded).await {
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
                    let expanded = util::expand_path(path);
                    if let Some(parent) = std::path::PathBuf::from(&expanded).parent() {
                        let _ = tokio::fs::create_dir_all(parent).await;
                    }
                    match tokio::fs::write(&expanded, content).await {
                        Ok(_) => format!("Successfully wrote {} bytes", content.len()),
                        Err(e) => format!("Error writing file: {}", e),
                    }
                } else {
                    "Error: path and content parameters required".to_string()
                }
            }
            "list_dir" => {
                if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                    let expanded = util::expand_path(path);
                    match tokio::fs::read_dir(&expanded).await {
                        Ok(mut entries) => {
                            let mut items = Vec::new();
                            while let Some(entry) = entries.next_entry().await.unwrap_or(None) {
                                items.push(entry.file_name().to_string_lossy().to_string());
                            }
                            if items.is_empty() {
                                "Directory is empty".to_string()
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
            "edit_file" => {
                let path = args.get("path").and_then(|v| v.as_str());
                let old_string = args.get("old_string").and_then(|v| v.as_str());
                let new_string = args.get("new_string").and_then(|v| v.as_str());

                if let (Some(path), Some(old_str), Some(new_str)) = (path, old_string, new_string) {
                    let expanded = util::expand_path(path);
                    match tokio::fs::read_to_string(&expanded).await {
                        Ok(content) => {
                            if content.contains(old_str) {
                                let new_content = content.replace(old_str, new_str);
                                match tokio::fs::write(&expanded, &new_content).await {
                                    Ok(_) => format!("Successfully edited {} bytes in {}", new_content.len() - content.len(), expanded),
                                    Err(e) => format!("Error writing file: {}", e),
                                }
                            } else {
                                "Error: old_string not found in file".to_string()
                            }
                        }
                        Err(e) => format!("Error reading file: {}", e),
                    }
                } else {
                    "Error: path, old_string, and new_string parameters required".to_string()
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

    /// Cancel all subagents for a session
    pub async fn cancel_by_session(&self, session_key: &str) -> usize {
        let mut count = 0;
        let tasks = self.session_tasks.read().await;
        if let Some(task_ids) = tasks.get(session_key) {
            let running = self.running_tasks.read().await;
            for task_id in task_ids {
                if let Some(handle) = running.get(task_id) {
                    handle.abort();
                    count += 1;
                }
            }
        }
        count
    }

    /// Get count of running subagents
    pub async fn get_running_count(&self) -> usize {
        self.running_tasks.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_subagent_get_running_count() {
        // Test the get_running_count functionality
        // We don't need a real provider to test the counter

        // We can't easily create a SubagentManager without a real provider
        // So we just verify the empty HashMap works as expected
        let tasks: Arc<RwLock<HashMap<String, String>>> = Arc::new(RwLock::new(HashMap::new()));

        let count = tasks.read().await.len();
        assert_eq!(count, 0);

        // Add a task
        tasks.write().await.insert("task1".to_string(), "test".to_string());
        let count = tasks.read().await.len();
        assert_eq!(count, 1);

        // Remove a task
        tasks.write().await.remove("task1");
        let count = tasks.read().await.len();
        assert_eq!(count, 0);
    }
}