//! MCP Transport Layer
//!
//! Provides stdio and HTTP transport implementations for MCP

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt};
use tokio::sync::Mutex;

/// Transport trait for MCP
#[async_trait]
pub trait Transport: Send + Sync {
    /// Read a JSON-RPC message
    async fn read(&mut self) -> Result<Option<Value>>;

    /// Write a JSON-RPC message
    async fn write(&mut self, message: Value) -> Result<()>;
}

/// Stdio transport for MCP
pub struct StdioTransport {
    reader: tokio::io::Lines<tokio::io::BufReader<tokio::io::Stdin>>,
    writer: Arc<Mutex<tokio::io::Stdout>>,
}

impl StdioTransport {
    pub fn new() -> Self {
        let stdin = tokio::io::stdin();
        let reader = tokio::io::BufReader::new(stdin);
        let lines = reader.lines();
        let stdout = tokio::io::stdout();
        Self {
            reader: lines,
            writer: Arc::new(Mutex::new(stdout)),
        }
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn read(&mut self) -> Result<Option<Value>> {
        match self.reader.next_line().await {
            Ok(Some(line)) => {
                let value: Value = serde_json::from_str(&line)?;
                Ok(Some(value))
            }
            Ok(None) => Ok(None), // EOF
            Err(e) => Err(anyhow::anyhow!("Read error: {}", e)),
        }
    }

    async fn write(&mut self, message: Value) -> Result<()> {
        let mut writer = self.writer.lock().await;
        let line = serde_json::to_string(&message)?;
        writer.write_all(line.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
        Ok(())
    }
}

/// In-memory transport for testing
pub struct InMemoryTransport {
    read_queue: Arc<Mutex<Vec<Value>>>,
    write_queue: Arc<Mutex<Vec<Value>>>,
}

impl InMemoryTransport {
    pub fn new() -> Self {
        Self {
            read_queue: Arc::new(Mutex::new(Vec::new())),
            write_queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_messages(messages: Vec<Value>) -> Self {
        Self {
            read_queue: Arc::new(Mutex::new(messages)),
            write_queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn push_message(&self, message: Value) {
        let mut queue = self.read_queue.lock().await;
        queue.push(message);
    }

    pub async fn get_written_messages(&self) -> Vec<Value> {
        let queue = self.write_queue.lock().await;
        queue.clone()
    }
}

#[async_trait]
impl Transport for InMemoryTransport {
    async fn read(&mut self) -> Result<Option<Value>> {
        let mut queue = self.read_queue.lock().await;
        Ok(queue.pop())
    }

    async fn write(&mut self, message: Value) -> Result<()> {
        let mut queue = self.write_queue.lock().await;
        queue.push(message);
        Ok(())
    }
}

/// HTTP transport for MCP server
pub struct HttpTransport {
    request_body: Value,
}

impl HttpTransport {
    pub fn new(body: Value) -> Self {
        Self { request_body: body }
    }

    pub fn into_request_body(self) -> Value {
        self.request_body
    }
}

#[async_trait]
impl Transport for HttpTransport {
    async fn read(&mut self) -> Result<Option<Value>> {
        Ok(Some(std::mem::take(&mut self.request_body)))
    }

    async fn write(&mut self, _message: Value) -> Result<()> {
        // For HTTP, writing is handled separately
        Ok(())
    }
}

/// Mock transport for testing that stores sent messages
pub struct MockTransport {
    sent_messages: Arc<Mutex<Vec<Value>>>,
    response_index: Arc<Mutex<usize>>,
    responses: Vec<Value>,
}

impl MockTransport {
    pub fn new(responses: Vec<Value>) -> Self {
        Self {
            sent_messages: Arc::new(Mutex::new(Vec::new())),
            response_index: Arc::new(Mutex::new(0)),
            responses,
        }
    }

    pub async fn get_sent_messages(&self) -> Vec<Value> {
        let messages = self.sent_messages.lock().await;
        messages.clone()
    }
}

#[async_trait]
impl Transport for MockTransport {
    async fn read(&mut self) -> Result<Option<Value>> {
        let mut index = self.response_index.lock().await;
        if *index < self.responses.len() {
            let response = self.responses[*index].clone();
            *index += 1;
            Ok(Some(response))
        } else {
            Ok(None)
        }
    }

    async fn write(&mut self, message: Value) -> Result<()> {
        let mut messages = self.sent_messages.lock().await;
        messages.push(message);
        Ok(())
    }
}
