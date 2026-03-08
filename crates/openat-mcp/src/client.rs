//! MCP Client Implementation
//!
//! Provides MCP client functionality for connecting to external MCP servers

use crate::types::*;
use anyhow::Result;
use serde_json::Value;

/// HTTP MCP Client that connects to HTTP endpoints
#[derive(Clone)]
pub struct HttpMcpClient {
    base_url: String,
    client: reqwest::Client,
    server_info: Option<ServerInfo>,
    server_capabilities: Option<ServerCapabilities>,
}

impl HttpMcpClient {
    /// Create a new HTTP MCP client
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
            server_info: None,
            server_capabilities: None,
        }
    }

    /// Send a JSON-RPC request and get the response
    async fn send_request(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params
        });

        let url = format!("{}/mcp", self.base_url);
        let response = self.client.post(&url)
            .json(&request)
            .send()
            .await?
            .json::<Value>()
            .await?;

        // Check for error
        if let Some(error) = response.get("error") {
            let error: JsonRpcError = serde_json::from_value(error.clone())?;
            anyhow::bail!("MCP error: {} (code: {})", error.message, error.code);
        }

        let result = response.get("result")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No result in response"))?;

        Ok(result)
    }

    /// Initialize the connection
    pub async fn initialize(&mut self) -> Result<()> {
        let params = serde_json::json!({
            "protocol_version": "2024-11-05",
            "capabilities": {},
            "client_info": {
                "name": "openat-mcp-client",
                "version": "0.1.0"
            }
        });

        let result = self.send_request("initialize", Some(params)).await?;

        let init_result: InitializeResult = serde_json::from_value(result)?;

        self.server_info = Some(init_result.server_info);
        self.server_capabilities = Some(init_result.capabilities);

        tracing::info!("Connected to MCP server: {} v{}",
            self.server_info.as_ref().map_or("unknown", |s| s.name.as_str()),
            self.server_info.as_ref().map_or("unknown", |s| s.version.as_str())
        );

        Ok(())
    }

    /// List available tools
    pub async fn list_tools(&mut self) -> Result<Vec<McpTool>> {
        // Initialize if not already done
        if self.server_info.is_none() {
            self.initialize().await?;
        }

        let result = self.send_request("tools/list", None).await?;
        let tools_result: ToolsListResult = serde_json::from_value(result)?;

        Ok(tools_result.tools)
    }

    /// Call a tool
    pub async fn call_tool(&mut self, name: &str, arguments: Option<Value>) -> Result<ToolCallResult> {
        // Initialize if not already done
        if self.server_info.is_none() {
            self.initialize().await?;
        }

        let params = serde_json::json!({
            "name": name,
            "arguments": arguments.unwrap_or(Value::Null)
        });

        let result = self.send_request("tools/call", Some(params)).await?;
        let tool_result: ToolCallResult = serde_json::from_value(result)?;

        Ok(tool_result)
    }

    /// Get server info
    pub fn server_info(&self) -> Option<&ServerInfo> {
        self.server_info.as_ref()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_mcp_client_creation() {
        // Test placeholder
    }
}
