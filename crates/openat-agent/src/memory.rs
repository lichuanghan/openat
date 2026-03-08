//! Memory system for persistent agent memory
//!
//! Two-layer memory:
//! - MEMORY.md: Long-term facts and important information
//! - HISTORY.md: Grep-searchable conversation log

use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use serde_json::json;

use openat_providers::LLMProvider;
use crate::Message;

/// Memory store for persistent storage
#[derive(Clone)]
pub struct MemoryStore {
    #[allow(dead_code)]
    workspace: PathBuf,
    /// Directory for memory files
    memory_dir: PathBuf,
    /// Long-term memory file
    memory_file: PathBuf,
    /// Searchable history file
    history_file: PathBuf,
}

impl MemoryStore {
    /// Create a new memory store
    pub fn new(workspace: PathBuf) -> Self {
        let memory_dir = workspace.join("memory");
        let memory_file = memory_dir.join("MEMORY.md");
        let history_file = memory_dir.join("HISTORY.md");

        Self {
            workspace,
            memory_dir,
            memory_file,
            history_file,
        }
    }

    /// Ensure memory directory exists
    pub async fn ensure_dir(&self) -> anyhow::Result<()> {
        tokio::fs::create_dir_all(&self.memory_dir).await?;
        Ok(())
    }

    /// Read long-term memory
    pub async fn read_long_term(&self) -> anyhow::Result<String> {
        match tokio::fs::read_to_string(&self.memory_file).await {
            Ok(content) => Ok(content),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
            Err(e) => Err(e.into()),
        }
    }

    /// Write long-term memory
    pub async fn write_long_term(&self, content: &str) -> anyhow::Result<()> {
        tokio::fs::write(&self.memory_file, content).await?;
        Ok(())
    }

    /// Append to history
    pub async fn append_history(&self, entry: &str) -> anyhow::Result<()> {
        use tokio::io::AsyncWriteExt;

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.history_file)
            .await?;

        file.write_all(entry.as_bytes()).await?;
        file.write_all(b"\n\n").await?;
        Ok(())
    }

    /// Get memory context for the agent
    pub async fn get_memory_context(&self) -> String {
        match self.read_long_term().await {
            Ok(memory) if !memory.is_empty() => {
                format!("## Long-term Memory\n{}", memory)
            }
            _ => String::new(),
        }
    }

    /// Add a message to history
    pub async fn add_to_history(&self, role: &str, content: &str, tools_used: &[String]) {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M").to_string();
        let tools_str = if tools_used.is_empty() {
            String::new()
        } else {
            format!(" [tools: {}]", tools_used.join(", "))
        };

        let entry = format!("[{}] {}{}: {}", timestamp, role.to_uppercase(), tools_str, content);

        if let Err(e) = self.append_history(&entry).await {
            tracing::warn!("Failed to append to history: {}", e);
        }
    }

    /// Consolidate old messages into memory
    pub async fn consolidate(
        &self,
        messages: &[Message],
        provider: &Arc<dyn LLMProvider>,
        model: &str,
        last_consolidated: usize,
        memory_window: usize,
    ) -> bool {
        // Check if consolidation is needed
        let keep_count = memory_window / 2;
        if messages.len() <= keep_count {
            return true;
        }

        // Get messages to consolidate (after last consolidated point)
        let old_messages: Vec<_> = messages
            .iter()
            .skip(last_consolidated)
            .take(messages.len() - keep_count)
            .collect();

        if old_messages.is_empty() {
            return true;
        }

        tracing::info!("Memory consolidation: {} messages to consolidate", old_messages.len());

        // Build conversation to process
        let mut lines = Vec::new();
        for msg in &old_messages {
            if msg.content.is_empty() {
                continue;
            }
            let tools_str = if msg.tool_calls.is_empty() {
                String::new()
            } else {
                let tools: Vec<String> = msg.tool_calls.iter().map(|t| t.name.clone()).collect();
                format!(" [tools: {}]", tools.join(", "))
            };
            lines.push(format!(
                "[{}] {}{}: {}",
                Utc::now().format("%Y-%m-%d %H:%M"),
                format!("{:?}", msg.role).to_uppercase(),
                tools_str,
                msg.content
            ));
        }

        let conversation = lines.join("\n");

        // Get current memory
        let current_memory = match self.read_long_term().await {
            Ok(m) => m,
            Err(_) => String::new(),
        };

        let prompt = format!(
            r#"Process this conversation and call the save_memory tool with your consolidation.

## Current Long-term Memory
{}

## Conversation to Process
{}"#,
            if current_memory.is_empty() { "(empty)" } else { &current_memory },
            conversation
        );

        // Call LLM to consolidate memory
        let save_memory_tool = json!({
            "type": "function",
            "function": {
                "name": "save_memory",
                "description": "Save the memory consolidation result to persistent storage.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "history_entry": {
                            "type": "string",
                            "description": "A paragraph (2-5 sentences) summarizing key events/decisions/topics. Start with [YYYY-MM-DD HH:MM]. Include detail useful for grep search."
                        },
                        "memory_update": {
                            "type": "string",
                            "description": "Full updated long-term memory as markdown. Include all existing facts plus new ones. Return unchanged if nothing new."
                        }
                    },
                    "required": ["history_entry", "memory_update"]
                }
            }
        });

        let tool_defs: Vec<serde_json::Value> = vec![save_memory_tool];

        let messages_json: Vec<serde_json::Value> = vec![
            json!({
                "role": "system",
                "content": "You are a memory consolidation agent. Call the save_memory tool with your consolidation of the conversation."
            }),
            json!({
                "role": "user",
                "content": prompt
            }),
        ];

        match provider.chat(&messages_json, model, &tool_defs).await {
            Ok(response) => {
                if response.tool_calls.is_empty() {
                    tracing::warn!("Memory consolidation: LLM did not call save_memory");
                    return false;
                }

                // Process the tool call
                let tool_call = &response.tool_calls[0];
                if tool_call.name != "save_memory" {
                    tracing::warn!("Memory consolidation: Wrong tool called: {}", tool_call.name);
                    return false;
                }

                // Parse arguments
                let args = if let Some(arg_str) = tool_call.arguments.get("history_entry") {
                    // Arguments are already parsed as Value
                    let hist_entry = arg_str.as_str().unwrap_or("");
                    let mem_update = tool_call.arguments.get("memory_update")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    (hist_entry.to_string(), mem_update.to_string())
                } else {
                    // Try to parse from string
                    let args_str = serde_json::to_string(&tool_call.arguments).unwrap_or_default();
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&args_str) {
                        (
                            parsed.get("history_entry").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            parsed.get("memory_update").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        )
                    } else {
                        tracing::warn!("Memory consolidation: Failed to parse tool arguments");
                        return false;
                    }
                };

                // Save memory and history
                if !args.1.is_empty() && args.1 != "(unchanged)" {
                    if let Err(e) = self.write_long_term(&args.1).await {
                        tracing::error!("Failed to write long-term memory: {}", e);
                    }
                }

                if !args.0.is_empty() {
                    if let Err(e) = self.append_history(&args.0).await {
                        tracing::error!("Failed to append history: {}", e);
                    }
                }

                tracing::info!("Memory consolidation completed");
                true
            }
            Err(e) => {
                tracing::error!("Memory consolidation failed: {}", e);
                false
            }
        }
    }
}
