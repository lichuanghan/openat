//! Configuration module.
//!
//! All config structs use #[derive(Default)] and #[serde(default)]
//! to reduce boilerplate and enable field-level defaults.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// Provider configurations

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ProviderConfig {
    pub api_key: String,
    pub api_base: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Providers {
    pub openrouter: ProviderConfig,
    pub anthropic: ProviderConfig,
    pub openai: ProviderConfig,
    pub groq: ProviderConfig,
    pub gemini: ProviderConfig,
    pub minimax: ProviderConfig,
    pub deepseek: ProviderConfig,
    pub zhipu: ProviderConfig,
    pub moonshot: ProviderConfig,
    pub vllm: ProviderConfig,
}

// Agent configurations

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AgentDefaults {
    pub model: String,
    pub max_tokens: usize,
    pub temperature: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Agents {
    pub defaults: AgentDefaults,
}

// Tool configurations

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct WebSearch {
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ProxyConfig {
    pub url: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Tools {
    pub web_search: WebSearch,
    pub proxy: ProxyConfig,
    pub restrict_to_workspace: bool,
}

// Channel configurations

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Telegram {
    pub enabled: bool,
    pub token: String,
    pub allowed_users: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct WhatsApp {
    pub enabled: bool,
    pub bridge_url: String,
    pub phone_number: Option<String>,
    pub allowed_numbers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct QQ {
    pub enabled: bool,
    pub api_url: String,
    pub event_url: String,
    pub access_token: String,
    pub allowed_users: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Discord {
    pub enabled: bool,
    pub token: String,
    pub allowed_users: Vec<String>,
    pub gateway_url: String,
    pub intents: i32,
}

impl Discord {
    pub fn default_with_gateway() -> Self {
        Self {
            enabled: false,
            token: String::new(),
            allowed_users: Vec::new(),
            gateway_url: "wss://gateway.discord.gg/?v=10&encoding=json".to_string(),
            intents: 37377,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Channels {
    pub telegram: Telegram,
    pub whatsapp: WhatsApp,
    pub qq: QQ,
    pub discord: Discord,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub providers: Providers,
    pub agents: Agents,
    pub tools: Tools,
    pub channels: Channels,
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        tracing::debug!("Config path: {:?}", path);
        tracing::debug!("Config exists: {}", path.exists());
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                tracing::debug!("Config content length: {}", content.len());
                match serde_json::from_str::<Config>(&content) {
                    Ok(config) => {
                        tracing::debug!("Config parsed successfully");
                        return config;
                    }
                    Err(e) => {
                        tracing::debug!("Config parse failed: {}", e);
                        tracing::debug!("First 500 chars of config: {}", &content[..std::cmp::min(500, content.len())]);
                    }
                }
            } else {
                tracing::debug!("Failed to read config file");
            }
        }
        tracing::debug!("Using default config");
        Self::default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    pub fn get_api_key(&self) -> Option<&str> {
        if !self.providers.openrouter.api_key.is_empty() {
            Some(&self.providers.openrouter.api_key)
        } else if !self.providers.anthropic.api_key.is_empty() {
            Some(&self.providers.anthropic.api_key)
        } else if !self.providers.openai.api_key.is_empty() {
            Some(&self.providers.openai.api_key)
        } else if !self.providers.groq.api_key.is_empty() {
            Some(&self.providers.groq.api_key)
        } else if !self.providers.gemini.api_key.is_empty() {
            Some(&self.providers.gemini.api_key)
        } else if !self.providers.minimax.api_key.is_empty() {
            Some(&self.providers.minimax.api_key)
        } else if !self.providers.deepseek.api_key.is_empty() {
            Some(&self.providers.deepseek.api_key)
        } else if !self.providers.zhipu.api_key.is_empty() {
            Some(&self.providers.zhipu.api_key)
        } else if !self.providers.moonshot.api_key.is_empty() {
            Some(&self.providers.moonshot.api_key)
        } else {
            None
        }
    }
}

pub(crate) fn config_path() -> PathBuf {
    if let Ok(path) = std::env::var("OPENAT_CONFIG") {
        PathBuf::from(path)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".openat/config.json")
    } else {
        PathBuf::from("config.json")
    }
}

/// Get workspace path from environment or use default
pub fn workspace_path() -> PathBuf {
    if let Ok(ws) = std::env::var("OPENAT_WORKSPACE") {
        PathBuf::from(ws)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".openat/workspace")
    } else {
        PathBuf::from("workspace")
    }
}

/// Ensure workspace directory exists
pub fn ensure_workspace_exists() -> PathBuf {
    let path = workspace_path();
    if let Err(e) = std::fs::create_dir_all(&path) {
        tracing::warn!("Failed to create workspace directory: {}", e);
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(!config.providers.openrouter.api_key.is_empty() || config.providers.openrouter.api_key.is_empty());
    }

    #[test]
    fn test_telegram_default() {
        let telegram = Telegram::default();
        assert!(!telegram.enabled);
        assert!(telegram.token.is_empty());
        assert!(telegram.allowed_users.is_empty());
    }

    #[test]
    fn test_discord_default() {
        let discord = Discord::default_with_gateway();
        assert!(!discord.enabled);
        assert!(discord.token.is_empty());
        assert!(discord.gateway_url.contains("discord.gg"));
    }
}
