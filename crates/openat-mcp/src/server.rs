//! MCP Server Implementation
//!
//! Provides MCP server functionality for exposing openat tools

use crate::transport::Transport;
use crate::types::*;
use anyhow::Result;
use openat_tools::prelude::ToolRegistry;
use serde_json::Value;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

/// MCP Server
pub struct McpServer {
    tool_registry: Arc<Mutex<ToolRegistry>>,
    capabilities: ServerCapabilities,
    server_info: ServerInfo,
    initialized: bool,
}

impl McpServer {
    pub fn new(tool_registry: ToolRegistry) -> Self {
        Self {
            tool_registry: Arc::new(Mutex::new(tool_registry)),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: true }),
                resources: None,
                prompts: None,
            },
            server_info: ServerInfo::new("openat", "0.1.0"),
            initialized: false,
        }
    }

    /// Handle a JSON-RPC request
    pub async fn handle_request(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone();

        // Handle methods that work before initialization
        match request.method.as_str() {
            "initialize" => {
                let params: InitializeParams = match serde_json::from_value(request.params.unwrap_or(Value::Null)) {
                    Ok(p) => p,
                    Err(e) => {
                        return JsonRpcResponse::error(id, JsonRpcError::invalid_params(&format!("Invalid initialize params: {}", e)));
                    }
                };

                tracing::info!("MCP Client initialized: {} v{}", params.client_info.name, params.client_info.version);
                self.initialized = true;

                let mut result = InitializeResult::new(&self.server_info.name, &self.server_info.version);
                result.capabilities = self.capabilities.clone();

                // Send initialized notification (async, fire and forget)
                // In a real implementation, we'd send this via the transport

                JsonRpcResponse::success(id, serde_json::to_value(result).unwrap())
            }
            "tools/list" => {
                if !self.initialized {
                    return JsonRpcResponse::error(id, JsonRpcError::invalid_request());
                }
                self.handle_tools_list(id).await
            }
            "tools/call" => {
                if !self.initialized {
                    return JsonRpcResponse::error(id, JsonRpcError::invalid_request());
                }
                self.handle_tools_call(id, request.params).await
            }
            "ping" => {
                JsonRpcResponse::success(id, serde_json::json!({ "pong": true }))
            }
            _ => {
                if !self.initialized && request.method != "initialize" {
                    JsonRpcResponse::error(id, JsonRpcError::invalid_request())
                } else {
                    JsonRpcResponse::error(id, JsonRpcError::method_not_found())
                }
            }
        }
    }

    async fn handle_tools_list(&self, id: Option<Value>) -> JsonRpcResponse {
        let registry = self.tool_registry.lock().await;
        let definitions = registry.definitions().await;

        let tools: Vec<McpTool> = definitions
            .into_iter()
            .map(|def| {
                McpTool::new(&def.name, &def.description, def.parameters)
            })
            .collect();

        tracing::debug!("Listing {} tools", tools.len());

        let result = ToolsListResult {
            tools,
            _meta: None,
        };

        JsonRpcResponse::success(id, serde_json::to_value(result).unwrap())
    }

    async fn handle_tools_call(&self, id: Option<Value>, params: Option<Value>) -> JsonRpcResponse {
        let params: CallToolParams = match params {
            Some(p) => match serde_json::from_value(p) {
                Ok(p) => p,
                Err(e) => {
                    return JsonRpcResponse::error(id, JsonRpcError::invalid_params(&format!("Invalid call tool params: {}", e)));
                }
            },
            None => {
                return JsonRpcResponse::error(id, JsonRpcError::invalid_params("Missing params"));
            }
        };

        let name = params.name;
        let arguments = params.arguments.map(|a| a.to_string()).unwrap_or_default();

        tracing::info!("Calling tool: {} with args: {}", name, arguments);

        let registry = self.tool_registry.lock().await;
        match registry.execute(&name, &arguments).await {
            Ok(result) => {
                let tool_result = ToolCallResult {
                    content: vec![ToolContent::text(&result)],
                    is_error: None,
                };
                JsonRpcResponse::success(id, serde_json::to_value(tool_result).unwrap())
            }
            Err(e) => {
                let tool_result = ToolCallResult {
                    content: vec![ToolContent::text(&format!("Error: {}", e))],
                    is_error: Some(true),
                };
                JsonRpcResponse::success(id, serde_json::to_value(tool_result).unwrap())
            }
        }
    }
}

/// Run MCP server with stdio transport
pub async fn run_stdio_server(tool_registry: ToolRegistry) -> Result<()> {
    use crate::transport::StdioTransport;

    tracing::info!("Starting MCP server with stdio transport");

    let mut server = McpServer::new(tool_registry);
    let mut transport = StdioTransport::new();

    loop {
        match transport.read().await {
            Ok(Some(request)) => {
                let request: JsonRpcRequest = match serde_json::from_value(request) {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::error!("Failed to parse request: {}", e);
                        let response = JsonRpcResponse::error(None, JsonRpcError::parse_error());
                        transport.write(serde_json::to_value(response).unwrap()).await.ok();
                        continue;
                    }
                };

                tracing::debug!("Received request: {}", request.method);
                let response = server.handle_request(request).await;
                transport.write(serde_json::to_value(response).unwrap()).await?;
            }
            Ok(None) => {
                tracing::info!("Stdin closed, shutting down");
                break;
            }
            Err(e) => {
                tracing::error!("Read error: {}", e);
                break;
            }
        }
    }

    Ok(())
}

/// Run MCP server with HTTP transport
pub async fn run_http_server(port: u16, tool_registry: ToolRegistry) -> Result<()> {
    use std::net::SocketAddr;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    tracing::info!("Starting MCP server on port {}", port);

    // Create shared server state
    let server_state = Arc::new(Mutex::new(McpServer::new(tool_registry)));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    loop {
        let (stream, addr) = listener.accept().await?;
        tracing::debug!("New connection from {}", addr);

        let state = server_state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_http_connection(stream, state).await {
                tracing::error!("Connection error: {}", e);
            }
        });
    }
}

async fn handle_http_connection(
    mut stream: tokio::net::TcpStream,
    server: Arc<Mutex<McpServer>>,
) -> Result<()> {
    use tokio::io::AsyncReadExt;

    let mut buffer = vec![0u8; 65536];
    let bytes_read = stream.read(&mut buffer).await?;

    if bytes_read == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..bytes_read]);

    // Simple HTTP parser
    let (method, path, body) = parse_http_request(&request)?;

    let response = match (method.as_str(), path.as_str()) {
        ("POST", "/mcp") | ("POST", "/") => {
            // Handle MCP JSON-RPC request
            let request: JsonRpcRequest = match serde_json::from_str(&body) {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("Failed to parse JSON-RPC request: {}", e);
                    let resp = JsonRpcResponse::error(None, JsonRpcError::parse_error());
                    return send_json_response(&mut stream, serde_json::to_string(&resp)?).await;
                }
            };

            let mut srv = server.lock().await;
            let rpc_response = srv.handle_request(request).await;

            send_json_response(&mut stream, serde_json::to_string(&rpc_response)?).await?
        }
        ("GET", "/sse") | ("GET", "/events") => {
            // SSE endpoint - for now return a simple response
            send_sse_response(&mut stream).await?
        }
        ("GET", "/health") => {
            let health = serde_json::json!({
                "status": "ok",
                "server": "openat-mcp"
            });
            send_json_response(&mut stream, health.to_string()).await?
        }
        _ => {
            send_json_response(&mut stream, r#"{"error": "Not found"}"#.to_string()).await?
        }
    };

    Ok(response)
}

fn parse_http_request(request: &str) -> Result<(String, String, String)> {
    let mut lines = request.lines();

    let request_line = lines.next().ok_or_else(|| anyhow::anyhow!("Empty request"))?;
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        anyhow::bail!("Invalid request line");
    }

    let method = parts[0].to_string();
    let path = parts[1].to_string();

    // Find body (after blank line)
    let body_start = request.find("\r\n\r\n").map(|i| i + 4)
        .or_else(|| request.find("\n\n").map(|i| i + 2));

    let body = match body_start {
        Some(pos) => request[pos..].to_string(),
        None => String::new(),
    };

    Ok((method, path, body))
}

async fn send_json_response(
    stream: &mut tokio::net::TcpStream,
    body: String,
) -> Result<()> {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes()).await?;
    Ok(())
}

async fn send_sse_response(
    stream: &mut tokio::net::TcpStream,
) -> Result<()> {
    // Send SSE response with content-type text/event-stream
    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\n\r\n";
    stream.write_all(response.as_bytes()).await?;

    // Keep connection alive and send initial message
    let init = "data: {\"event\": \"initialized\"}\n\n";
    stream.write_all(init.as_bytes()).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsonrpc_request() {
        let req = JsonRpcRequest::new("tools/list", None);
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "tools/list");
    }

    #[test]
    fn test_jsonrpc_response() {
        let resp = JsonRpcResponse::success(None, serde_json::json!({"tools": []}));
        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.result.is_some());
    }
}
