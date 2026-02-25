//! Tool plugin system for openat
//!
//! Provides dynamic tool loading and registration.

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Tool definition with metadata
#[derive(Debug, Clone)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub category: ToolCategory,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ToolCategory {
    #[default]
    General,
    Filesystem,
    Network,
    Developer,
    Custom(String),
}

/// Plugin metadata
#[derive(Debug, Clone)]
pub struct PluginMeta {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
}

/// Plugin trait for dynamic tool loading
#[async_trait::async_trait]
pub trait Plugin: Send + Sync {
    /// Plugin metadata
    fn meta(&self) -> PluginMeta;

    /// Tools provided by this plugin
    fn tools(&self) -> Vec<ToolSpec>;

    /// Initialize the plugin
    async fn init(&self, config: &serde_json::Value) -> Result<(), String> {
        let _ = config;
        Ok(())
    }
}

/// Tool registry for managing plugins and tools
#[derive(Clone)]
pub struct ToolRegistry {
    plugins: Arc<RwLock<HashMap<String, Box<dyn Plugin>>>>,
    tools: Arc<RwLock<HashMap<String, ToolSpec>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a plugin
    pub async fn register_plugin(&self, plugin: Box<dyn Plugin>) -> Result<(), String> {
        let meta = plugin.meta();
        let tools = plugin.tools();
        let tool_count = tools.len();

        // Register tools
        let mut tools_lock = self.tools.write().await;
        for tool in tools {
            if tool.enabled {
                tools_lock.insert(tool.name.clone(), tool);
            }
        }

        // Register plugin
        let mut plugins_lock = self.plugins.write().await;
        plugins_lock.insert(meta.id.clone(), plugin);

        // Log registration (commented out - requires tracing dependency)
        // tracing::info!("Registered plugin: {} with {} tools", meta.name, tool_count);
        Ok(())
    }

    /// Get all enabled tools
    pub async fn get_tools(&self) -> Vec<ToolSpec> {
        let tools_lock = self.tools.read().await;
        tools_lock.values().cloned().collect()
    }

    /// Get tool by name
    pub async fn get_tool(&self, name: &str) -> Option<ToolSpec> {
        let tools_lock = self.tools.read().await;
        tools_lock.get(name).cloned()
    }

    /// Enable/disable a tool
    pub async fn set_tool_enabled(&self, name: &str, enabled: bool) -> bool {
        let mut tools_lock = self.tools.write().await;
        if let Some(tool) = tools_lock.get_mut(name) {
            tool.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// List registered plugins
    pub async fn list_plugins(&self) -> Vec<PluginMeta> {
        let plugins_lock = self.plugins.read().await;
        plugins_lock.values().map(|p| p.meta()).collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Built-in tools registry
pub mod builtins {
    use super::*;

    /// Get default tool specifications
    pub fn default_tools() -> Vec<ToolSpec> {
        vec![
            ToolSpec {
                name: "read_file".to_string(),
                description: "Read the contents of a file".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "The file path to read" }
                    },
                    "required": ["path"]
                }),
                category: ToolCategory::Filesystem,
                enabled: true,
            },
            ToolSpec {
                name: "write_file".to_string(),
                description: "Write content to a file".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "The file path to write" },
                        "content": { "type": "string", "description": "The content to write" }
                    },
                    "required": ["path", "content"]
                }),
                category: ToolCategory::Filesystem,
                enabled: true,
            },
            ToolSpec {
                name: "list_dir".to_string(),
                description: "List directory contents".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "The directory path" }
                    },
                    "required": ["path"]
                }),
                category: ToolCategory::Filesystem,
                enabled: true,
            },
            ToolSpec {
                name: "exec".to_string(),
                description: "Execute a shell command".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "cmd": { "type": "string", "description": "The command to execute" }
                    },
                    "required": ["cmd"]
                }),
                category: ToolCategory::Developer,
                enabled: true,
            },
            ToolSpec {
                name: "web_search".to_string(),
                description: "Search the web for information".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "The search query" }
                    },
                    "required": ["query"]
                }),
                category: ToolCategory::Network,
                enabled: true,
            },
            ToolSpec {
                name: "web_fetch".to_string(),
                description: "Fetch content from a URL".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "url": { "type": "string", "description": "The URL to fetch" }
                    },
                    "required": ["url"]
                }),
                category: ToolCategory::Network,
                enabled: true,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry() {
        let registry = ToolRegistry::new();
        assert!(registry.get_tools().await.is_empty());

        // Enable a tool
        registry.set_tool_enabled("test", true).await;
        assert!(registry.get_tool("test").await.is_none()); // Not registered yet
    }
}
