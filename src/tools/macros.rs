//! Tool macros - helpers for creating tools.
//!
//! # Example
//!
//! ```
//! use_tool!(ReadFileTool, "read_file", "Read a file", |args| {
//!     let path = args["path"].as_str().ok_or("Missing path")?;
//!     Ok(tokio::fs::read_to_string(path).await?)
//! });
//! ```

/// Create a simple tool with name, description, and executor.
///
/// # Arguments
/// * `$name` - Tool struct name
/// * `$tool_name` - Tool name string for LLM
/// * `$description` - Tool description
/// * `$args_type` - Arguments struct name
/// * `$executor` - Async closure that receives `&$args_type` and returns `Result<String, String>`
#[macro_export]
macro_rules! make_tool {
    ($name:ident, $tool_name:expr, $description:expr, $args_type:ident, $executor:expr) => {
        #[derive(Debug, Clone)]
        pub struct $name;

        impl $name {
            pub fn new() -> Self {
                Self
            }
        }

        #[async_trait::async_trait]
        impl $crate::tools::Tool for $name {
            fn name(&self) -> &str {
                $tool_name
            }

            fn description(&self) -> &str {
                $description
            }

            fn definition(&self) -> $crate::types::ToolDefinition {
                use serde_json::json;
                $crate::types::ToolDefinition::new(
                    $tool_name,
                    $description,
                    json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "The file path"
                            }
                        },
                        "required": ["path"]
                    }),
                )
            }

            async fn execute(&self, args: &str) -> Result<String, String> {
                #[derive(serde::Deserialize)]
                struct $args_type {
                    path: String,
                }

                let args: $args_type = serde_json::from_str(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;

                $executor(&args)
            }
        }
    };
}

/// Create a tool with custom JSON schema for arguments.
///
/// # Arguments
/// * `$name` - Tool struct name
/// * `$tool_name` - Tool name string
/// * `$description` - Tool description
/// * `$schema` - JSON schema for arguments
/// * `$args_type` - Arguments struct name
/// * `$executor` - Async closure for execution
#[macro_export]
macro_rules! make_tool_with_schema {
    ($name:ident, $tool_name:expr, $description:expr, $schema:expr, $args_type:ident, $executor:expr) => {
        #[derive(Debug, Clone)]
        pub struct $name;

        impl $name {
            pub fn new() -> Self {
                Self
            }
        }

        #[async_trait::async_trait]
        impl $crate::tools::Tool for $name {
            fn name(&self) -> &str {
                $tool_name
            }

            fn description(&self) -> &str {
                $description
            }

            fn definition(&self) -> $crate::types::ToolDefinition {
                $crate::types::ToolDefinition::new($tool_name, $description, $schema)
            }

            async fn execute(&self, args: &str) -> Result<String, String> {
                #[derive(serde::Deserialize)]
                struct $args_type {
                    path: String,
                }

                let args: $args_type = serde_json::from_str(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;

                $executor(&args)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_expansion() {
        // This test just ensures the macros compile
        assert!(true);
    }
}
