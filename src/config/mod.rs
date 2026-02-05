use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub api_key: String,
    pub api_base: Option<String>,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            api_base: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Providers {
    pub openrouter: ProviderConfig,
    pub anthropic: ProviderConfig,
    pub openai: ProviderConfig,
    pub groq: ProviderConfig,
    pub gemini: ProviderConfig,
    pub minimax: ProviderConfig,
}

impl Default for Providers {
    fn default() -> Self {
        Self {
            openrouter: ProviderConfig::default(),
            anthropic: ProviderConfig::default(),
            openai: ProviderConfig::default(),
            groq: ProviderConfig::default(),
            gemini: ProviderConfig::default(),
            minimax: ProviderConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefaults {
    pub model: String,
    pub max_tokens: usize,
    pub temperature: f64,
}

impl Default for AgentDefaults {
    fn default() -> Self {
        Self {
            model: "anthropic/claude-opus-4-5".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agents {
    pub defaults: AgentDefaults,
}

impl Default for Agents {
    fn default() -> Self {
        Self {
            defaults: AgentDefaults::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearch {
    pub api_key: String,
}

impl Default for WebSearch {
    fn default() -> Self {
        Self {
            api_key: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tools {
    pub web_search: WebSearch,
}

impl Default for Tools {
    fn default() -> Self {
        Self {
            web_search: WebSearch::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Telegram {
    pub enabled: bool,
    pub token: String,
    pub allowed_users: Vec<String>,
}

impl Default for Telegram {
    fn default() -> Self {
        Self {
            enabled: false,
            token: String::new(),
            allowed_users: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channels {
    pub telegram: Telegram,
    pub whatsapp: WhatsApp,
    pub qq: QQ,
}

impl Default for Channels {
    fn default() -> Self {
        Self {
            telegram: Telegram::default(),
            whatsapp: WhatsApp::default(),
            qq: QQ::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsApp {
    pub enabled: bool,
    pub bridge_url: String,
    pub phone_number: Option<String>,
    pub allowed_numbers: Vec<String>,
}

impl Default for WhatsApp {
    fn default() -> Self {
        Self {
            enabled: false,
            bridge_url: "ws://localhost:3001".to_string(),
            phone_number: None,
            allowed_numbers: Vec::new(),
        }
    }
}

/// QQ channel configuration (via OneBot protocol)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QQ {
    pub enabled: bool,
    /// OneBot HTTP API URL (e.g., http://localhost:3000)
    pub api_url: String,
    /// OneBot WebSocket event URL (e.g., ws://localhost:3000)
    pub event_url: String,
    /// OneBot access token
    pub access_token: String,
    /// Allowed QQ user IDs
    pub allowed_users: Vec<String>,
}

impl Default for QQ {
    fn default() -> Self {
        Self {
            enabled: false,
            api_url: "http://localhost:3000".to_string(),
            event_url: "ws://localhost:3000".to_string(),
            access_token: String::new(),
            allowed_users: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub providers: Providers,
    pub agents: Agents,
    pub tools: Tools,
    pub channels: Channels,
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
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
        } else {
            None
        }
    }

    pub fn get_api_base(&self) -> Option<&str> {
        if !self.providers.openrouter.api_key.is_empty() {
            Some("https://openrouter.ai/api/v1")
        } else {
            None
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            providers: Providers::default(),
            agents: Agents::default(),
            tools: Tools::default(),
            channels: Channels::default(),
        }
    }
}

pub fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".nanobot")
        .join("config.json")
}

pub fn workspace_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".nanobot")
        .join("workspace")
}

pub fn ensure_workspace_exists() -> PathBuf {
    let path = workspace_path();
    let _ = fs::create_dir_all(&path);
    path
}
