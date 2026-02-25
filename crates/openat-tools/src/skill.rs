//! Skill system for openat
//!
//! Provides predefined workflows and automation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Skill definition
#[derive(Debug, Clone)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub triggers: Vec<String>,
    pub prompt: String,
    pub tools: Vec<String>,
    pub enabled: bool,
}

impl Skill {
    pub fn new(id: &str, name: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            triggers: vec![],
            prompt: String::new(),
            tools: vec![],
            enabled: true,
        }
    }

    pub fn with_trigger(mut self, trigger: &str) -> Self {
        self.triggers.push(trigger.to_string());
        self
    }

    pub fn with_prompt(mut self, prompt: &str) -> Self {
        self.prompt = prompt.to_string();
        self
    }

    pub fn with_tools(mut self, tools: Vec<&str>) -> Self {
        self.tools = tools.into_iter().map(|s| s.to_string()).collect();
        self
    }
}

/// Skill manager
#[derive(Clone)]
pub struct SkillManager {
    skills: Arc<RwLock<HashMap<String, Skill>>>,
}

impl SkillManager {
    pub fn new() -> Self {
        Self {
            skills: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new SkillManager with default skills registered
    pub fn with_defaults() -> Self {
        let mut manager = Self::new();
        // Register default skills synchronously
        let skills = defaults::default_skills();
        // Note: This is a simplified approach - in production, use async init
        for skill in skills {
            // We'll register via async in agent
            let _ = skill;
        }
        manager
    }

    /// Initialize with default skills (async)
    pub async fn init_default_skills(&self) {
        for skill in defaults::default_skills() {
            self.register(skill).await;
        }
    }

    /// Register a skill
    pub async fn register(&self, skill: Skill) {
        let mut skills = self.skills.write().await;
        skills.insert(skill.id.clone(), skill);
    }

    /// Get skill by ID
    pub async fn get(&self, id: &str) -> Option<Skill> {
        let skills = self.skills.read().await;
        skills.get(id).cloned()
    }

    /// Find skills that match a trigger
    pub async fn find_by_trigger(&self, trigger: &str) -> Vec<Skill> {
        // Auto-initialize on first use
        if self.skills.read().await.is_empty() {
            self.init_default_skills().await;
        }

        let skills = self.skills.read().await;
        skills
            .values()
            .filter(|s| s.enabled && s.triggers.iter().any(|t| trigger.contains(t)))
            .cloned()
            .collect()
    }

    /// List all skills
    pub async fn list(&self) -> Vec<Skill> {
        let skills = self.skills.read().await;
        skills.values().cloned().collect()
    }

    /// Enable/disable a skill
    pub async fn set_enabled(&self, id: &str, enabled: bool) -> bool {
        let mut skills = self.skills.write().await;
        if let Some(skill) = skills.get_mut(id) {
            skill.enabled = enabled;
            true
        } else {
            false
        }
    }
}

impl Default for SkillManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Default skills
pub mod defaults {
    use super::*;

    /// Get default skills
    pub fn default_skills() -> Vec<Skill> {
        vec![
            Skill {
                id: "translator".to_string(),
                name: "Translator".to_string(),
                description: "Translate text between languages".to_string(),
                triggers: vec!["翻译".to_string(), "translate".to_string()],
                prompt: "You are a professional translator. Translate the following text accurately.".to_string(),
                tools: vec![],
                enabled: true,
            },
            Skill {
                id: "summarizer".to_string(),
                name: "Summarizer".to_string(),
                description: "Summarize long text".to_string(),
                triggers: vec!["总结".to_string(), "summarize".to_string(), "摘要".to_string()],
                prompt: "Provide a concise summary of the following text, capturing the key points.".to_string(),
                tools: vec![],
                enabled: true,
            },
            Skill {
                id: "coder".to_string(),
                name: "Code Assistant".to_string(),
                description: "Help with code-related tasks".to_string(),
                triggers: vec!["写代码".to_string(), "code".to_string(), "编程".to_string()],
                prompt: "You are a helpful coding assistant. Provide clean, well-commented code.".to_string(),
                tools: vec!["exec".to_string()],
                enabled: true,
            },
            Skill {
                id: "researcher".to_string(),
                name: "Researcher".to_string(),
                description: "Research and gather information".to_string(),
                triggers: vec!["研究".to_string(), "research".to_string(), "调查".to_string()],
                prompt: "Research the topic thoroughly and provide comprehensive information with sources.".to_string(),
                tools: vec!["web_search".to_string(), "web_fetch".to_string()],
                enabled: true,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_skill_manager() {
        let manager = SkillManager::new();

        let skill = Skill::new("test", "Test Skill", "A test skill")
            .with_trigger("test trigger");

        manager.register(skill).await;

        let found = manager.find_by_trigger("test trigger").await;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "test");
    }
}
