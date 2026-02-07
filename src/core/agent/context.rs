//! Context builder for assembling agent prompts.
//!
//! Builds system prompts from bootstrap files, memory, skills, and conversation history.

use crate::core::agent::memory::MemoryManager;
use crate::core::agent::skills::SkillManager;
use crate::config::workspace_path;
use std::fs;
use std::path::PathBuf;

/// Bootstrap files to load for system prompt
const BOOTSTRAP_FILES: &[&str] = &["AGENTS.md", "SOUL.md", "USER.md", "TOOLS.md", "IDENTITY.md"];

/// Context builder for agent prompts
#[derive(Debug)]
pub struct ContextBuilder {
    workspace: PathBuf,
    memory: MemoryManager,
    skills: SkillManager,
}

impl ContextBuilder {
    /// Create a new context builder
    pub fn new() -> Self {
        let workspace = workspace_path();

        Self {
            workspace: workspace.clone(),
            memory: MemoryManager::new(&workspace),
            skills: SkillManager::new(&workspace),
        }
    }

    /// Build the complete system prompt
    pub async fn build_system_prompt(&self, _skill_names: Option<Vec<String>>) -> String {
        let mut parts = Vec::new();

        // Core identity
        parts.push(self.get_identity());

        // Bootstrap files
        if let Some(bootstrap) = self.load_bootstrap_files() {
            parts.push(bootstrap);
        }

        // Memory context
        let memory_context = self.memory.get_context();
        if !memory_context.is_empty() {
            parts.push(format!("# Memory\n\n{}", memory_context));
        }

        // Skills - progressive loading
        // Always-loaded skills: include full content
        let always_content = self.skills.get_always_load();
        if !always_content.is_empty() {
            parts.push(format!("# Active Skills\n\n{}", always_content.join("\n\n")));
        }

        // Available skills: only show summary
        let skills_summary = self.build_skills_summary();
        if !skills_summary.is_empty() {
            let summary = format!(
                r#"# Skills

The following skills extend your capabilities. To use a skill, read its SKILL.md file using the read_file tool.

{}"#,
                skills_summary
            );
            parts.push(summary);
        }

        parts.join("\n\n---\n\n")
    }

    /// Get the core identity section
    pub fn get_identity(&self) -> String {
        use chrono::Utc;
        use std::env;

        let now = Utc::now();
        let date_str = now.format("%Y-%m-%d %H:%M (%A)").to_string();

        let os = env::consts::OS;
        let arch = env::consts::ARCH;
        let runtime = format!("{} {}", os, arch);

        let workspace_path = self.workspace.display().to_string();
        let memory_path = format!("{}/memory/MEMORY.md", workspace_path);
        let daily_path = format!("{}/memory/YYYY-MM-DD.md", workspace_path);
        let skills_path = format!("{}/skills/{{skill-name}}/SKILL.md", workspace_path);

        format!(
            r#"# openat 🤖

You are openat, a helpful AI assistant. You have access to tools that allow you to:
- Read, write, and edit files
- Execute shell commands
- Search the web and fetch web pages
- Send messages to users on chat channels
- Schedule reminders and recurring tasks

## Current Time
{}

## Runtime
{}

## Workspace
Your workspace is at: {}
- Memory files: {}
- Daily notes: {}
- Custom skills: {}

IMPORTANT: When responding to direct questions or conversations, reply directly with your text response.
Only use the 'message' tool when you need to send a message to a specific chat channel (like Telegram, WhatsApp).
For normal conversation, just respond with text - do not call the message tool.

Always be helpful, accurate, and concise. When using tools, explain what you're doing.
When remembering something, write to {}"#,
            date_str,
            runtime,
            workspace_path,
            memory_path,
            daily_path,
            skills_path,
            memory_path
        )
    }

    /// Load all bootstrap files from workspace
    fn load_bootstrap_files(&self) -> Option<String> {
        let mut parts = Vec::new();

        for filename in BOOTSTRAP_FILES {
            let file_path = self.workspace.join(filename);
            if file_path.exists() {
                if let Ok(content) = fs::read_to_string(&file_path) {
                    parts.push(format!("## {}\n\n{}", filename, content));
                }
            }
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n\n"))
        }
    }

    /// Build a summary of available skills
    pub fn build_skills_summary(&self) -> String {
        let optional_skills = self.skills.get_optional();

        if optional_skills.is_empty() {
            return String::new();
        }

        let summaries: Vec<String> = optional_skills
            .iter()
            .map(|s| {
                format!(
                    "- **{}**: {}",
                    s.name,
                    s.description
                )
            })
            .collect();

        summaries.join("\n")
    }
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}
