//! Tool registry

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::{Tool, ToolDefinition, ToolResult};

/// Registry for managing tools
#[derive(Clone)]
pub struct ToolRegistry {
    tools: Arc<Mutex<HashMap<String, Arc<dyn Tool>>>>,
}

impl ToolRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            tools: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a tool
    pub async fn register<T: Tool + 'static>(&self, tool: T) {
        let mut tools = self.tools.lock().await;
        tools.insert(tool.name().to_string(), Arc::new(tool));
    }

    /// Get a tool by name
    pub async fn get(&self, name: &str) -> Option<std::sync::Arc<dyn Tool>> {
        let tools = self.tools.lock().await;
        tools.get(name).cloned()
    }

    /// Get all tool definitions
    pub async fn definitions(&self) -> Vec<ToolDefinition> {
        let tools = self.tools.lock().await;
        tools.values()
            .map(|t| t.definition())
            .collect()
    }

    /// Get all tool names
    pub async fn names(&self) -> Vec<String> {
        let tools = self.tools.lock().await;
        tools.keys().cloned().collect()
    }

    /// Execute a tool by name
    pub async fn execute(&self, name: &str, args: &str) -> ToolResult {
        if let Some(tool) = self.get(name).await {
            tool.execute(args).await
        } else {
            Err(format!("Tool '{}' not found", name))
        }
    }

    /// Check if a tool exists
    pub async fn contains(&self, name: &str) -> bool {
        let tools = self.tools.lock().await;
        tools.contains_key(name)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
