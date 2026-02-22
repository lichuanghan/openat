//! Tool types

use serde_json::{json, Value};

/// Tool definition for LLM
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

impl ToolDefinition {
    pub fn new(name: &str, description: &str, parameters: Value) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            parameters,
        }
    }
}

/// Tool call from LLM
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

impl ToolCall {
    pub fn new(id: &str, name: &str, arguments: Value) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            arguments,
        }
    }
}

/// Result of tool execution
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub content: ToolResultContent,
}

impl ToolResult {
    pub fn success(tool_call_id: &str, content: &str) -> Self {
        Self {
            tool_call_id: tool_call_id.to_string(),
            content: ToolResultContent::Text(content.to_string()),
        }
    }

    pub fn error(tool_call_id: &str, error: &str) -> Self {
        Self {
            tool_call_id: tool_call_id.to_string(),
            content: ToolResultContent::Error(error.to_string()),
        }
    }
}

/// Content of tool result
#[derive(Debug, Clone)]
pub enum ToolResultContent {
    Text(String),
    Error(String),
    Json(Value),
}

impl ToolResultContent {
    pub fn is_error(&self) -> bool {
        matches!(self, ToolResultContent::Error(_))
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            ToolResultContent::Text(s) => Some(s),
            _ => None,
        }
    }
}

/// Convert tool call to LLM format
impl From<&ToolCall> for serde_json::Value {
    fn from(call: &ToolCall) -> Self {
        json!({
            "role": "tool",
            "tool_call_id": call.id,
            "name": call.name,
            "content": call.arguments
        })
    }
}

/// Convert tool result to LLM format
impl From<&ToolResult> for serde_json::Value {
    fn from(result: &ToolResult) -> Self {
        match &result.content {
            ToolResultContent::Text(text) => json!({
                "role": "tool",
                "tool_call_id": result.tool_call_id,
                "content": text
            }),
            ToolResultContent::Error(error) => json!({
                "role": "tool",
                "tool_call_id": result.tool_call_id,
                "content": format!("Error: {}", error)
            }),
            ToolResultContent::Json(json) => json!({
                "role": "tool",
                "tool_call_id": result.tool_call_id,
                "content": json
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_definition_new() {
        let def = ToolDefinition::new(
            "echo",
            "Echo back the input",
            json!({"type": "object", "properties": {}})
        );
        assert_eq!(def.name, "echo");
        assert_eq!(def.description, "Echo back the input");
    }

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult::success("call123", "Hello");
        assert_eq!(result.tool_call_id, "call123");
        assert!(!result.content.is_error());
        assert_eq!(result.content.as_text(), Some("Hello"));
    }

    #[test]
    fn test_tool_result_error() {
        let result = ToolResult::error("call123", "Not found");
        assert_eq!(result.tool_call_id, "call123");
        assert!(result.content.is_error());
    }
}
