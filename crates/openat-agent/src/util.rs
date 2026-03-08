//! Utility functions for openat-agent

// ============================================================================
// Constants
// ============================================================================

/// Common channel names
pub mod channel {
    pub const CLI: &str = "cli";
    pub const DISCORD: &str = "discord";
    pub const CRON: &str = "cron";
}

/// Common chat targets
pub mod target {
    pub const DIRECT: &str = "direct";
    pub const CRON: &str = "cron";
}

/// Message roles
pub mod role {
    pub const USER: &str = "user";
    pub const ASSISTANT: &str = "assistant";
    pub const SYSTEM: &str = "system";
    pub const TOOL: &str = "tool";
}

/// Expand ~ to home directory
pub fn expand_path(path: &str) -> String {
    if path.starts_with('~') {
        match std::env::var("HOME") {
            Ok(home) => path.replacen('~', &home, 1),
            Err(_) => path.to_string(),
        }
    } else {
        path.to_string()
    }
}

/// Parse tool arguments from JSON string or object
pub fn parse_tool_arguments(arguments: &serde_json::Value) -> std::collections::HashMap<String, serde_json::Value> {
    use std::collections::HashMap;

    if arguments.is_string() {
        let arg_str = arguments.as_str().unwrap_or("");
        if arg_str.starts_with('{') {
            serde_json::from_str(arg_str).unwrap_or_default()
        } else {
            HashMap::new()
        }
    } else if let Some(obj) = arguments.as_object() {
        obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    } else {
        HashMap::new()
    }
}
