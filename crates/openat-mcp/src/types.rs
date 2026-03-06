//! MCP Protocol Types
//!
//! Defines the core types for the Model Context Protocol (MCP)

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC version
pub const JSONRPC_VERSION: &str = "2.0";

/// JSON-RPC Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    pub fn new(method: &str, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: None,
            method: method.to_string(),
            params,
        }
    }

    pub fn with_id(mut self, id: Value) -> Self {
        self.id = Some(id);
        self
    }
}

/// JSON-RPC Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<Value>, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// JSON-RPC Error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    pub fn new(code: i32, message: &str) -> Self {
        Self {
            code,
            message: message.to_string(),
            data: None,
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Parse error (-32700)
    pub fn parse_error() -> Self {
        Self::new(-32700, "Parse error")
    }

    /// Invalid request (-32600)
    pub fn invalid_request() -> Self {
        Self::new(-32600, "Invalid Request")
    }

    /// Method not found (-32601)
    pub fn method_not_found() -> Self {
        Self::new(-32601, "Method not found")
    }

    /// Invalid params (-32602)
    pub fn invalid_params(msg: &str) -> Self {
        Self::new(-32602, msg)
    }

    /// Internal error (-32603)
    pub fn internal_error() -> Self {
        Self::new(-32603, "Internal error")
    }
}

/// Server capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerCapabilities {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,
}

/// Tools capability
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolsCapability {
    #[serde(default)]
    pub list_changed: bool,
}

/// Resources capability
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourcesCapability {
    #[serde(default)]
    pub subscribe: bool,
    #[serde(default)]
    pub list_changed: bool,
}

/// Prompts capability
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptsCapability;

/// Client capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientCapabilities {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,
}

/// Server info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

impl ServerInfo {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
        }
    }
}

/// Initialize result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: ServerInfo,
}

impl InitializeResult {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities::default(),
            server_info: ServerInfo::new(name, version),
        }
    }
}

/// MCP Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

impl McpTool {
    pub fn new(name: &str, description: &str, input_schema: Value) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            input_schema,
        }
    }
}

/// Tools list result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsListResult {
    pub tools: Vec<McpTool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Tool call arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallArguments {
    pub name: String,
    pub arguments: Value,
}

/// Tool call result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub content: Vec<ToolContent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// Tool content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolContent {
    Text { #[serde(rename = "type")] content_type: String, text: String },
    Image { #[serde(rename = "type")] content_type: String, data: String, mime_type: String },
    Resource { #[serde(rename = "type")] content_type: String, uri: String, mime_type: Option<String>, text: Option<String> },
}

impl ToolContent {
    pub fn text(text: &str) -> Self {
        Self::Text {
            content_type: "text".to_string(),
            text: text.to_string(),
        }
    }
}

/// Initialize request params
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    pub protocol_version: Option<String>,
    pub capabilities: ClientCapabilities,
    pub client_info: ClientInfo,
}

/// Client info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

/// Tool call request params
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolParams {
    pub name: String,
    #[serde(default)]
    pub arguments: Option<Value>,
}

/// Resources list result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesListResult {
    pub resources: Vec<Resource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "mimeType", default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}
