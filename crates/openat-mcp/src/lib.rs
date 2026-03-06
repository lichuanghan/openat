//! OpenAT MCP (Model Context Protocol) crate
//!
//! Provides MCP Server and Client implementations for the openat project.
//!
//! ## MCP Server
//!
//! Expose openat tools to MCP clients (like Cursor, Claude Desktop):
//!
//! ```ignore
//! use openat_mcp::server::run_stdio_server;
//! use openat_tools::prelude::ToolRegistry;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let registry = ToolRegistry::new();
//!     // Register tools...
//!     run_stdio_server(registry).await?;
//!     Ok(())
//! }
//! ```
//!
//! ## MCP Client
//!
//! Connect to external MCP servers:
//!
//! ```ignore
//! use openat_mcp::client::HttpMcpClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let client = HttpMcpClient::new("http://localhost:8080");
//!     let tools = client.list_tools().await?;
//!     for tool in tools {
//!         println!("{}: {}", tool.name, tool.description);
//!     }
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod server;
pub mod transport;
pub mod types;

// Re-export commonly used types
pub use client::HttpMcpClient;
pub use server::McpServer;
pub use transport::{InMemoryTransport, StdioTransport};
pub use types::*;
