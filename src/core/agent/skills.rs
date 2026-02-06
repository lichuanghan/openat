//! Skills system - load and manage agent skills.
//!
//! Skills are reusable capabilities that can be enabled/disabled.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tokio::fs as async_fs;

/// Skill metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub always_load: bool,
    pub requires: Option<SkillRequirements>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRequirements {
    pub bins: Vec<String>,
    pub env: Vec<String>,
}

/// A skill that the agent can use
#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub content: String,
    pub always_load: bool,
    pub path: PathBuf,
}

impl Skill {
    /// Load a skill from a directory
    pub async fn load(path: &Path) -> Option<Self> {
        let skill_file = path.join("SKILL.md");

        if !skill_file.exists() {
            return None;
        }

        let content = match fs::read_to_string(&skill_file) {
            Ok(c) => c,
            Err(_) => return None,
        };

        // Parse metadata from the content
        let (metadata, content) = Self::parse_metadata(&content)?;

        Some(Self {
            name: metadata.name,
            description: metadata.description,
            content,
            always_load: metadata.always_load,
            path: path.to_path_buf(),
        })
    }

    /// Parse YAML frontmatter
    fn parse_metadata(content: &str) -> Option<(SkillMetadata, String)> {
        // Simple frontmatter parsing
        if !content.starts_with("---") {
            return None;
        }

        let end_marker = content[3..].find("---")?;
        let yaml_part = &content[3..3 + end_marker];
        let rest = &content[3 + end_marker + 3..];

        let metadata: SkillMetadata = match serde_yaml::from_str(yaml_part) {
            Ok(m) => m,
            Err(_) => return None,
        };

        Some((metadata, rest.trim().to_string()))
    }

    /// Get the skill content for context
    pub fn to_context(&self) -> String {
        format!(
            r#"## Skill: {}

{}{}"#,
            self.name,
            self.description,
            self.content
        )
    }
}

/// Skill loader and manager
#[derive(Debug)]
pub struct SkillManager {
    workspace_skills: PathBuf,
    always_load: Vec<Skill>,
    optional: Vec<Skill>,
}

impl SkillManager {
    pub fn new(workspace: &PathBuf) -> Self {
        let workspace_skills = workspace.join("skills");

        Self {
            workspace_skills,
            always_load: Vec::new(),
            optional: Vec::new(),
        }
    }

    /// Load all skills from workspace
    pub async fn load_all(&mut self) {
        if !self.workspace_skills.exists() {
            return;
        }

        let entries = match fs::read_dir(&self.workspace_skills) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Some(skill) = Skill::load(&entry.path()).await {
                    if skill.always_load {
                        self.always_load.push(skill);
                    } else {
                        self.optional.push(skill);
                    }
                }
            }
        }
    }

    /// Get skills that should always be loaded
    pub fn get_always_load(&self) -> Vec<String> {
        self.always_load
            .iter()
            .map(|s| s.to_context())
            .collect()
    }

    /// Get all optional skills (for listing)
    pub fn get_optional(&self) -> Vec<&Skill> {
        self.optional.iter().collect()
    }

    /// Get a skill by name
    pub fn get_skill(&self, name: &str) -> Option<&Skill> {
        self.optional.iter().find(|s| s.name == name)
    }
}
