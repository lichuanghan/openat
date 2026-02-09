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
    pub deepseek: ProviderConfig,
    pub zhipu: ProviderConfig,
    pub moonshot: ProviderConfig,
    pub vllm: ProviderConfig,
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
            deepseek: ProviderConfig::default(),
            zhipu: ProviderConfig::default(),
            moonshot: ProviderConfig::default(),
            vllm: ProviderConfig::default(),
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
pub struct ProxyConfig {
    pub url: String,
    pub enabled: bool,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tools {
    pub web_search: WebSearch,
    pub proxy: ProxyConfig,
    pub restrict_to_workspace: bool,
}

impl Default for Tools {
    fn default() -> Self {
        Self {
            web_search: WebSearch::default(),
            proxy: ProxyConfig::default(),
            restrict_to_workspace: false,
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
    pub discord: Discord,
}

impl Default for Channels {
    fn default() -> Self {
        Self {
            telegram: Telegram::default(),
            whatsapp: WhatsApp::default(),
            qq: QQ::default(),
            discord: Discord::default(),
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

/// Discord channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discord {
    pub enabled: bool,
    pub token: String,
    pub allowed_users: Vec<String>,
    pub gateway_url: String,
    pub intents: i32,
}

impl Default for Discord {
    fn default() -> Self {
        Self {
            enabled: false,
            token: String::new(),
            allowed_users: Vec::new(),
            gateway_url: "wss://gateway.discord.gg/?v=10&encoding=json".to_string(),
            intents: 37377, // GUILDS + GUILD_MESSAGES + DIRECT_MESSAGES + MESSAGE_CONTENT
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
        // Priority: OpenRouter > Anthropic > OpenAI > Groq > Gemini > MiniMax > DeepSeek > Zhipu > Moonshot
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
        } else if !self.providers.vllm.api_key.is_empty() {
            Some(&self.providers.vllm.api_key)
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

    /// Validate the configuration
    ///
    /// Returns a list of validation errors, if any.
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        // Validate agent defaults
        if self.agents.defaults.model.is_empty() {
            errors.push("Agent model cannot be empty".to_string());
        }
        if self.agents.defaults.max_tokens == 0 {
            errors.push("Agent max_tokens must be greater than 0".to_string());
        }
        if self.agents.defaults.temperature < 0.0 || self.agents.defaults.temperature > 2.0 {
            errors.push("Agent temperature must be between 0.0 and 2.0".to_string());
        }

        // Validate provider configurations
        if !self.providers.openrouter.api_key.is_empty() {
            if self.providers.openrouter.api_key.len() < 10 {
                errors.push("OpenRouter API key seems too short".to_string());
            }
        }
        if !self.providers.anthropic.api_key.is_empty() {
            if self.providers.anthropic.api_key.len() < 10 {
                errors.push("Anthropic API key seems too short".to_string());
            }
        }
        if !self.providers.openai.api_key.is_empty() {
            if self.providers.openai.api_key.len() < 10 {
                errors.push("OpenAI API key seems too short".to_string());
            }
        }
        if !self.providers.groq.api_key.is_empty() {
            if self.providers.groq.api_key.len() < 10 {
                errors.push("Groq API key seems too short".to_string());
            }
        }

        // Validate Telegram config
        if self.channels.telegram.enabled {
            if self.channels.telegram.token.is_empty() {
                errors.push("Telegram token cannot be empty when enabled".to_string());
            }
            if self.channels.telegram.token.len() < 10 {
                errors.push("Telegram token seems too short".to_string());
            }
        }

        // Validate WhatsApp config
        if self.channels.whatsapp.enabled {
            if self.channels.whatsapp.bridge_url.is_empty() {
                errors.push("WhatsApp bridge URL cannot be empty when enabled".to_string());
            }
        }

        // Validate QQ config
        if self.channels.qq.enabled {
            if self.channels.qq.api_url.is_empty() {
                errors.push("QQ API URL cannot be empty when enabled".to_string());
            }
            if self.channels.qq.event_url.is_empty() {
                errors.push("QQ event URL cannot be empty when enabled".to_string());
            }
        }

        // Validate web search
        if !self.tools.web_search.api_key.is_empty() {
            if self.tools.web_search.api_key.len() < 10 {
                errors.push("Web search API key seems too short".to_string());
            }
        }

        errors
    }

    /// Check if any LLM provider is configured
    pub fn has_llm_provider(&self) -> bool {
        self.get_api_key().is_some()
    }

    /// Check if any channel is enabled
    pub fn has_enabled_channel(&self) -> bool {
        self.channels.telegram.enabled
            || self.channels.whatsapp.enabled
            || self.channels.qq.enabled
            || self.channels.discord.enabled
    }

    /// Check if web search is configured
    pub fn has_web_search(&self) -> bool {
        !self.tools.web_search.api_key.is_empty()
    }

    /// Get proxy URL if enabled
    pub fn get_proxy_url(&self) -> Option<&str> {
        if self.tools.proxy.enabled && !self.tools.proxy.url.is_empty() {
            Some(&self.tools.proxy.url)
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
        .join(".openat")
        .join("config.json")
}

pub fn workspace_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openat")
        .join("workspace")
}

pub fn ensure_workspace_exists() -> PathBuf {
    let path = workspace_path();
    let _ = fs::create_dir_all(&path);
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.providers.openrouter.api_key.is_empty());
        assert_eq!(config.agents.defaults.model, "anthropic/claude-opus-4-5");
        assert!(!config.channels.telegram.enabled);
    }

    #[test]
    fn test_config_validate_empty_model() {
        let mut config = Config::default();
        config.agents.defaults.model = "".to_string();
        let errors = config.validate();
        // Empty model should trigger error
        assert!(errors.contains(&"Agent model cannot be empty".to_string()));
    }

    #[test]
    fn test_config_validate_model_temperature() {
        let mut config = Config::default();
        config.agents.defaults.model = "test-model".to_string();
        config.agents.defaults.temperature = 3.0; // Invalid

        let errors = config.validate();
        assert!(errors.iter().any(|e| e.contains("temperature")));
    }

    #[test]
    fn test_config_validate_api_key_short() {
        let mut config = Config::default();
        config.agents.defaults.model = "test-model".to_string();
        config.providers.openrouter.api_key = "short".to_string();

        let errors = config.validate();
        assert!(errors.iter().any(|e| e.contains("OpenRouter API key")));
    }

    #[test]
    fn test_config_validate_telegram_enabled_without_token() {
        let mut config = Config::default();
        config.agents.defaults.model = "test-model".to_string();
        config.channels.telegram.enabled = true;

        let errors = config.validate();
        assert!(errors.iter().any(|e| e.contains("Telegram token")));
    }

    #[test]
    fn test_config_validate_valid() {
        let mut config = Config::default();
        config.agents.defaults.model = "test-model".to_string();
        config.providers.openrouter.api_key = "sk-12345678901234567890".to_string();
        config.channels.telegram.token = "123456:ABCDEFGHIJKLMNOP".to_string();

        let errors = config.validate();
        // Should have no validation errors
        assert!(!errors.iter().any(|e| e.contains("Agent model")));
        assert!(!errors.iter().any(|e| e.contains("OpenRouter")));
    }

    #[test]
    fn test_config_has_llm_provider() {
        let mut config = Config::default();
        assert!(!config.has_llm_provider());

        config.providers.openrouter.api_key = "sk-test".to_string();
        assert!(config.has_llm_provider());
    }

    #[test]
    fn test_config_has_enabled_channel() {
        let config = Config::default();
        assert!(!config.has_enabled_channel());

        let mut config = Config::default();
        config.channels.telegram.enabled = true;
        assert!(config.has_enabled_channel());
    }

    #[test]
    fn test_config_has_web_search() {
        let config = Config::default();
        assert!(!config.has_web_search());

        let mut config = Config::default();
        config.tools.web_search.api_key = "test-key".to_string();
        assert!(config.has_web_search());
    }

    #[test]
    fn test_get_api_key_priority() {
        let mut config = Config::default();
        assert!(config.get_api_key().is_none());

        config.providers.openrouter.api_key = "openrouter-key".to_string();
        assert_eq!(config.get_api_key(), Some("openrouter-key"));

        config.providers.anthropic.api_key = "anthropic-key".to_string();
        // OpenRouter has priority
        assert_eq!(config.get_api_key(), Some("openrouter-key"));
    }

    #[test]
    fn test_get_api_base() {
        let mut config = Config::default();
        assert!(config.get_api_base().is_none());

        config.providers.openrouter.api_key = "test".to_string();
        assert_eq!(config.get_api_base(), Some("https://openrouter.ai/api/v1"));
    }
}
