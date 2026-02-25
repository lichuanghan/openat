//! Tool framework for openat
//!
//! Provides a registry-based tool system with support for
//! filesystem, web, shell, and other tools.

pub mod prelude {
    pub use super::{Tool, ToolResult};
    pub use super::registry::ToolRegistry;
    pub use super::file::FileTool;
    pub use super::exec::ExecTool;
    pub use super::web::WebTool;
    pub use super::plugin::{Plugin, PluginMeta, ToolRegistry as PluginRegistry, ToolSpec, ToolCategory};
    pub use super::skill::{Skill, SkillManager};
}

pub mod registry;
pub mod file;
pub mod exec;
pub mod web;
pub mod plugin;
pub mod skill;

use openat_types::ToolDefinition;

/// Tool trait - all tools implement this
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    /// Tool name
    fn name(&self) -> &str;

    /// Tool description
    fn description(&self) -> &str;

    /// Get tool definition for LLM
    fn definition(&self) -> ToolDefinition;

    /// Execute the tool
    async fn execute(&self, args: &str) -> Result<String, String>;
}

/// Result of tool execution
pub type ToolResult = Result<String, String>;
