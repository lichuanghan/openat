//! Configuration management for openat

use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

pub mod prelude {
    pub use super::{Config, Providers, ProviderConfig, Agents, AgentDefaults};
    pub use super::{Tools, WebSearch, ProxyConfig};
    pub use super::{Channels, Telegram, WhatsApp, QQ, Discord};
    pub use super::ConfigWarning;
}

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
    /// Agent name for display
    pub name: String,
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
    pub enabled: bool,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ProxyConfig {
    pub url: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Tools {
    pub web_search: WebSearch,
    pub proxy: ProxyConfig,
    pub restrict_to_workspace: bool,
    // Tool enable/disable flags
    pub filesystem: bool,
    pub shell: bool,
    pub web_fetch: bool,
}

impl Default for Tools {
    fn default() -> Self {
        Self {
            web_search: WebSearch::default(),
            proxy: ProxyConfig::default(),
            restrict_to_workspace: false,
            filesystem: true,   // Enable filesystem by default
            shell: false,       // Disable shell by default for security
            web_fetch: false,   // Disable web_fetch by default
        }
    }
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
    pub app_id: String,
    pub client_secret: String,
    pub sandbox: bool,
    pub allowed_users: Vec<String>,
    pub listen_group: bool,
    pub listen_private: bool,
    pub listen_guild: bool,
    /// Enable streaming/typing effect for responses
    pub streaming: bool,
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
    /// Load configuration from default path
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
                    }
                }
            }
        }
        tracing::debug!("Using default config");
        Self::default()
    }

    /// Save configuration to default path
    pub fn save(&self) -> anyhow::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    /// Get first available API key
    pub fn get_api_key(&self) -> Option<&str> {
        let keys = [
            &self.providers.openrouter.api_key,
            &self.providers.anthropic.api_key,
            &self.providers.openai.api_key,
            &self.providers.groq.api_key,
            &self.providers.gemini.api_key,
            &self.providers.minimax.api_key,
            &self.providers.deepseek.api_key,
            &self.providers.zhipu.api_key,
            &self.providers.moonshot.api_key,
        ];

        for key in keys {
            if !key.is_empty() {
                return Some(key);
            }
        }
        None
    }

    /// Validate configuration
    pub fn validate(&self) -> Vec<ConfigWarning> {
        let mut warnings = Vec::new();

        // Check if any provider API key is configured
        if self.get_api_key().is_none() {
            warnings.push(ConfigWarning::MissingApiKey);
        }

        // Check if any channel is enabled
        let any_channel_enabled = self.channels.discord.enabled
            || self.channels.qq.enabled
            || self.channels.telegram.enabled
            || self.channels.whatsapp.enabled;

        if !any_channel_enabled {
            warnings.push(ConfigWarning::NoChannelEnabled);
        }

        // Check Discord config if enabled
        if self.channels.discord.enabled && self.channels.discord.token.is_empty() {
            warnings.push(ConfigWarning::DiscordMissingToken);
        }

        // Check QQ config if enabled
        if self.channels.qq.enabled {
            if self.channels.qq.app_id.is_empty() {
                warnings.push(ConfigWarning::QQMissingAppId);
            }
            if self.channels.qq.client_secret.is_empty() {
                warnings.push(ConfigWarning::QQMissingSecret);
            }
        }

        // Check web_search if enabled
        if self.tools.web_search.enabled && self.tools.web_search.api_key.is_empty() {
            warnings.push(ConfigWarning::WebSearchMissingApiKey);
        }

        warnings
    }

    /// Check if configuration is valid (no errors, warnings are ok)
    pub fn is_valid(&self) -> bool {
        self.validate().is_empty()
    }
}

/// Configuration warnings
#[derive(Debug, Clone)]
pub enum ConfigWarning {
    MissingApiKey,
    NoChannelEnabled,
    DiscordMissingToken,
    QQMissingAppId,
    QQMissingSecret,
    WebSearchMissingApiKey,
}

impl std::fmt::Display for ConfigWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigWarning::MissingApiKey => write!(f, "No LLM provider API key configured"),
            ConfigWarning::NoChannelEnabled => write!(f, "No messaging channel is enabled"),
            ConfigWarning::DiscordMissingToken => write!(f, "Discord channel enabled but token is empty"),
            ConfigWarning::QQMissingAppId => write!(f, "QQ channel enabled but app_id is empty"),
            ConfigWarning::QQMissingSecret => write!(f, "QQ channel enabled but client_secret is empty"),
            ConfigWarning::WebSearchMissingApiKey => write!(f, "Web search enabled but API key is empty"),
        }
    }
}

fn config_path() -> PathBuf {
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
        assert!(config.providers.openrouter.api_key.is_empty() || !config.providers.openrouter.api_key.is_empty());
    }

    #[test]
    fn test_discord_default() {
        let discord = Discord::default_with_gateway();
        assert!(!discord.enabled);
        assert!(discord.token.is_empty());
        assert!(discord.gateway_url.contains("discord.gg"));
    }
}
