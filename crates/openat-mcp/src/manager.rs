//! MCP Client Manager
//!
//! Manages connections to multiple MCP servers

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{json, Value};
use tokio::sync::RwLock;

use openat_config::McpServer;
use crate::client::HttpMcpClient;
use crate::types::{McpTool, ToolContent};

/// MCP Server connection wrapper
pub struct McpConnection {
    pub name: String,
    pub client: HttpMcpClient,
    pub tools: Vec<McpTool>,
}

/// MCP Client Manager
pub struct McpManager {
    /// Active connections
    connections: Arc<RwLock<HashMap<String, McpConnection>>>,
}

impl McpManager {
    /// Create a new MCP manager
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Connect to all configured MCP servers
    pub async fn connect_all(&self, servers: &[McpServer]) -> Result<(), String> {
        for server in servers {
            if let Err(e) = self.connect(server).await {
                tracing::warn!("Failed to connect to MCP server '{}': {}", server.name, e);
            }
        }
        Ok(())
    }

    /// Connect to a single MCP server
    pub async fn connect(&self, server: &McpServer) -> Result<(), String> {
        let url = server.url.as_ref()
            .ok_or_else(|| "MCP server URL is required for HTTP transport".to_string())?;

        let mut client = HttpMcpClient::new(url);

        // Initialize connection
        if let Err(e) = client.initialize().await {
            return Err(format!("Failed to initialize MCP server: {}", e));
        }

        // List available tools
        let tools = client.list_tools().await
            .map_err(|e| format!("Failed to list MCP tools: {}", e))?;

        tracing::info!("Connected to MCP server '{}' with {} tools", server.name, tools.len());

        // Store connection
        let connection = McpConnection {
            name: server.name.clone(),
            client,
            tools,
        };

        self.connections.write().await
            .insert(server.name.clone(), connection);

        Ok(())
    }

    /// Get all available tools from all connections
    pub async fn get_all_tools(&self) -> Vec<McpTool> {
        let connections = self.connections.read().await;
        let mut all_tools = Vec::new();

        for (_, conn) in connections.iter() {
            for tool in &conn.tools {
                all_tools.push(tool.clone());
            }
        }

        all_tools
    }

    /// Get tool definitions for LLM
    pub async fn get_tool_definitions(&self) -> Vec<Value> {
        let tools = self.get_all_tools().await;

        tools.iter().map(|tool| {
            json!({
                "type": "function",
                "function": {
                    "name": format!("mcp_{}", tool.name),
                    "description": tool.description.clone(),
                    "parameters": if tool.input_schema.is_null() { json!({"type": "object"}) } else { tool.input_schema.clone() }
                }
            })
        }).collect()
    }

    /// Call an MCP tool
    pub async fn call_tool(&self, server_name: &str, tool_name: &str, arguments: Option<Value>) -> Result<String, String> {
        // Get client clone for the call
        let client = {
            let connections = self.connections.read().await;
            let conn = connections.get(server_name)
                .ok_or_else(|| format!("MCP server '{}' not found", server_name))?;
            conn.client.clone()
        };

        let mut client = client;
        let result = client.call_tool(tool_name, arguments).await
            .map_err(|e| format!("Failed to call MCP tool: {}", e))?;

        // Extract text content from result
        let content = result.content.iter()
            .filter_map(|item| {
                match item {
                    ToolContent::Text { text, .. } => Some(text.as_str()),
                    ToolContent::Resource { text, .. } => text.as_ref().map(|s| s.as_str()),
                    _ => None,
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(content)
    }

    /// Disconnect from all servers
    pub async fn disconnect_all(&self) {
        let mut connections = self.connections.write().await;
        connections.clear();
        tracing::info!("Disconnected from all MCP servers");
    }
}

impl Default for McpManager {
    fn default() -> Self {
        Self::new()
    }
}
