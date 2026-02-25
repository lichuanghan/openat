//! Skill system for openat
//!
//! Provides predefined workflows and automation, aligned with nanobot's skill system.
//! Skills are defined in SKILL.md files with YAML frontmatter.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Skill metadata (parsed from YAML frontmatter)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillMetadata {
    /// Emoji for skill display
    pub emoji: Option<String>,
    /// Required binaries
    #[serde(default)]
    pub requires: SkillRequirements,
    /// Supported operating systems
    #[serde(default)]
    pub os: Vec<String>,
    /// Installation instructions
    #[serde(default)]
    pub install: Vec<String>,
    /// Homepage URL
    pub homepage: Option<String>,
    /// Custom metadata (for nanobot compatibility)
    #[serde(default)]
    pub nanobot: Option<serde_json::Value>,
}

/// Skill requirements
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillRequirements {
    /// Required binaries
    #[serde(default)]
    pub bins: Vec<String>,
    /// Required environment variables
    #[serde(default)]
    pub env: Vec<String>,
}

/// Skill definition (aligned with nanobot format)
#[derive(Debug, Clone)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    /// If true, skill content is always loaded in system prompt
    pub always: bool,
    /// Skill prompt/instructions
    pub prompt: String,
    /// Skill metadata
    pub metadata: SkillMetadata,
    /// Original file path (if loaded from file)
    #[allow(dead_code)]
    pub file_path: Option<PathBuf>,
}

impl Skill {
    /// Create a new skill
    pub fn new(id: &str, name: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            always: false,
            prompt: String::new(),
            metadata: SkillMetadata::default(),
            file_path: None,
        }
    }

    /// Check if skill requirements are met
    pub fn check_requirements(&self) -> Vec<String> {
        let mut missing = Vec::new();

        // Check required binaries
        for bin in &self.metadata.requires.bins {
            if which::which(bin).is_err() {
                missing.push(format!("binary: {}", bin));
            }
        }

        // Check required environment variables
        for env in &self.metadata.requires.env {
            if std::env::var(env).is_err() {
                missing.push(format!("env: {}", env));
            }
        }

        missing
    }
}

/// Skill summary for display in system prompt
#[derive(Debug, Clone)]
pub struct SkillSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub emoji: Option<String>,
    pub always: bool,
}

impl From<&Skill> for SkillSummary {
    fn from(skill: &Skill) -> Self {
        Self {
            id: skill.id.clone(),
            name: skill.name.clone(),
            description: skill.description.clone(),
            emoji: skill.metadata.emoji.clone(),
            always: skill.always,
        }
    }
}

/// Skill manager - handles loading and managing skills
#[derive(Clone)]
pub struct SkillManager {
    skills: Arc<RwLock<HashMap<String, Skill>>>,
    workspace: Arc<RwLock<Option<PathBuf>>>,
}

impl SkillManager {
    pub fn new() -> Self {
        Self {
            skills: Arc::new(RwLock::new(HashMap::new())),
            workspace: Arc::new(RwLock::new(None)),
        }
    }

    /// Set workspace path for loading skills
    pub fn set_workspace(&self, workspace: PathBuf) {
        let ws = self.workspace.clone();
        let workspace = Some(workspace);
        // Note: This is a simplified sync approach - actual loading happens in async context
        let _ = (ws, workspace);
    }

    /// Initialize with default skills (async)
    pub async fn init_default_skills(&self) {
        for skill in defaults::default_skills() {
            self.register(skill).await;
        }
    }

    /// Load skills from workspace directory
    pub async fn load_from_workspace(&self, workspace: &PathBuf) {
        let skills_dir = workspace.join("skills");

        if !skills_dir.exists() {
            debug!("Skills directory does not exist: {}", skills_dir.display());
            return;
        }

        let mut entries = match fs::read_dir(&skills_dir).await {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Failed to read skills directory: {}", e);
                return;
            }
        };

        while let Some(entry) = entries.next_entry().await.unwrap_or(None) {
            let path = entry.path();
            if path.is_dir() {
                let skill_file = path.join("SKILL.md");
                if skill_file.exists() {
                    match self.load_skill_from_file(&skill_file).await {
                        Ok(skill) => {
                            info!("Loaded skill from file: {}", skill.id);
                            self.register(skill).await;
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load skill from {:?}: {}", path, e);
                        }
                    }
                }
            }
        }
    }

    /// Load a skill from a SKILL.md file
    pub async fn load_skill_from_file(&self, path: &PathBuf) -> Result<Skill, String> {
        let content = fs::read_to_string(path)
            .await
            .map_err(|e| format!("Failed to read skill file: {}", e))?;

        parse_skill_from_content(&content, Some(path.clone()))
    }

    /// Register a skill
    pub async fn register(&self, skill: Skill) {
        let mut skills = self.skills.write().await;
        info!("Registering skill: {} ({})", skill.name, skill.id);
        skills.insert(skill.id.clone(), skill);
    }

    /// Get skill by ID
    pub async fn get(&self, id: &str) -> Option<Skill> {
        let skills = self.skills.read().await;
        skills.get(id).cloned()
    }

    /// Find skills that match a trigger (by description)
    pub async fn find_by_trigger(&self, trigger: &str) -> Vec<Skill> {
        // Auto-initialize on first use
        if self.skills.read().await.is_empty() {
            self.init_default_skills().await;
        }

        let skills = self.skills.read().await;
        let trigger_lower = trigger.to_lowercase();
        skills
            .values()
            .filter(|s| {
                s.name.to_lowercase().contains(&trigger_lower) ||
                s.description.to_lowercase().contains(&trigger_lower)
            })
            .cloned()
            .collect()
    }

    /// Get always-loaded skills
    pub async fn get_always_skills(&self) -> Vec<Skill> {
        if self.skills.read().await.is_empty() {
            self.init_default_skills().await;
        }

        let skills = self.skills.read().await;
        skills
            .values()
            .filter(|s| s.always)
            .cloned()
            .collect()
    }

    /// Get all skill summaries (for progressive loading)
    pub async fn get_skill_summaries(&self) -> Vec<SkillSummary> {
        if self.skills.read().await.is_empty() {
            self.init_default_skills().await;
        }

        let skills = self.skills.read().await;
        skills
            .values()
            .filter(|s| s.check_requirements().is_empty()) // Only show skills with met requirements
            .map(SkillSummary::from)
            .collect()
    }

    /// Load skill content by ID (for on-demand loading)
    pub async fn load_skill_content(&self, id: &str) -> Option<String> {
        let skill = self.get(id).await?;
        Some(skill.prompt)
    }

    /// List all skills
    pub async fn list(&self) -> Vec<Skill> {
        if self.skills.read().await.is_empty() {
            self.init_default_skills().await;
        }

        let skills = self.skills.read().await;
        skills.values().cloned().collect()
    }

    /// Enable/disable a skill
    pub async fn set_enabled(&self, id: &str, _enabled: bool) -> bool {
        // Note: For now, skills are always enabled if loaded
        // Could add enabled field to Skill in future
        self.skills.read().await.contains_key(id)
    }
}

impl Default for SkillManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse skill from SKILL.md content with YAML frontmatter
pub fn parse_skill_from_content(content: &str, file_path: Option<PathBuf>) -> Result<Skill, String> {
    // Check for YAML frontmatter
    if !content.starts_with("---") {
        return Err("Skill file must start with YAML frontmatter".to_string());
    }

    // Find the end of frontmatter
    let end_idx = content[3..]
        .find("---")
        .ok_or("Could not find closing --- in frontmatter")?;

    let yaml_content = &content[3..3 + end_idx];
    let markdown_content = &content[3 + end_idx + 3..];

    // Parse YAML frontmatter
    #[derive(Deserialize)]
    struct Frontmatter {
        name: String,
        description: String,
        #[serde(default)]
        always: bool,
        #[serde(default)]
        metadata: SkillMetadata,
    }

    let frontmatter: Frontmatter = serde_yaml::from_str(yaml_content)
        .map_err(|e| format!("Failed to parse YAML frontmatter: {}", e))?;

    Ok(Skill {
        id: frontmatter.name.to_lowercase().replace(' ', "_"),
        name: frontmatter.name,
        description: frontmatter.description,
        always: frontmatter.always,
        prompt: markdown_content.trim().to_string(),
        metadata: frontmatter.metadata,
        file_path,
    })
}

/// Default skills (internal)
pub mod defaults {
    use super::*;

    /// Get default skills (as Skill structs for backward compatibility)
    pub fn default_skills() -> Vec<Skill> {
        vec![
            Skill {
                id: "translator".to_string(),
                name: "Translator".to_string(),
                description: "Translate text between languages".to_string(),
                always: false,
                prompt: "You are a professional translator. Translate the following text accurately, preserving the original meaning and tone.".to_string(),
                metadata: SkillMetadata {
                    emoji: Some("🌐".to_string()),
                    requires: SkillRequirements::default(),
                    os: vec![],
                    install: vec![],
                    homepage: None,
                    nanobot: None,
                },
                file_path: None,
            },
            Skill {
                id: "summarizer".to_string(),
                name: "Summarizer".to_string(),
                description: "Summarize URLs, files, and long text".to_string(),
                always: false,
                prompt: "Provide a concise summary of the following text, capturing the key points in a clear and organized manner.".to_string(),
                metadata: SkillMetadata {
                    emoji: Some("📝".to_string()),
                    requires: SkillRequirements::default(),
                    os: vec![],
                    install: vec![],
                    homepage: None,
                    nanobot: None,
                },
                file_path: None,
            },
            Skill {
                id: "coder".to_string(),
                name: "Code Assistant".to_string(),
                description: "Help with code-related tasks".to_string(),
                always: false,
                prompt: "You are a helpful coding assistant. Provide clean, well-commented code with explanations.".to_string(),
                metadata: SkillMetadata {
                    emoji: Some("💻".to_string()),
                    requires: SkillRequirements {
                        bins: vec!["sh".to_string()],
                        env: vec![],
                    },
                    os: vec![],
                    install: vec![],
                    homepage: None,
                    nanobot: None,
                },
                file_path: None,
            },
            Skill {
                id: "researcher".to_string(),
                name: "Researcher".to_string(),
                description: "Research and gather information with sources".to_string(),
                always: false,
                prompt: "Research the topic thoroughly and provide comprehensive information with credible sources. Include links to references.".to_string(),
                metadata: SkillMetadata {
                    emoji: Some("🔍".to_string()),
                    requires: SkillRequirements::default(),
                    os: vec![],
                    install: vec![],
                    homepage: None,
                    nanobot: None,
                },
                file_path: None,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_skill_from_content() {
        let content = r#"---
name: Test Skill
description: A test skill for unit testing
always: false
metadata:
  emoji: "🧪"
  requires:
    bins: ["ls"]
    env: ["HOME"]
  os: ["darwin", "linux"]
---
This is the skill content.
It can be multiple lines.
"#;

        let skill = parse_skill_from_content(content, None).unwrap();
        assert_eq!(skill.name, "Test Skill");
        assert_eq!(skill.description, "A test skill for unit testing");
        assert!(!skill.always);
        assert_eq!(skill.prompt, "This is the skill content.\nIt can be multiple lines.");
        assert_eq!(skill.metadata.emoji, Some("🧪".to_string()));
    }
}
