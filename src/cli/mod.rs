//! CLI module - command line interface for openat.

mod commands;

pub use commands::{
    agent,
    agent_interactive,
    channel_login, channel_status,
    cron_add, cron_enable, cron_list, cron_remove,
    gateway,
};

use crate::config::{self, Config};
use anyhow::Result;
use std::path::Path;

pub const LOGO: &str = r#"
    |\__/,|   (`\
  _.|o o  |_   ) )
 -(((---(((--------
"#;

/// Initialize configuration and workspace
pub async fn onboard() -> Result<()> {
    println!("{}", LOGO);
    println!("Initializing openat...\n");

    let workspace = config::ensure_workspace_exists();

    let config = Config::load();
    config.save()?;
    println!("[+] Created ~/.openat/config.json");

    create_template(&workspace, "AGENTS.md", "# Agent Instructions\n\nYou are a helpful AI assistant.")?;
    create_template(&workspace, "SOUL.md", "# Soul\n\nI am openat, an AI assistant.")?;
    create_template(&workspace, "USER.md", "# User\n\nInformation about the user.")?;
    create_template(&workspace.join("memory"), "MEMORY.md", "# Memory\n\nImportant information.")?;

    println!("[+] Created workspace at ~/.openat/workspace");
    println!("\n{}", LOGO);
    println!("Ready! Next steps:");
    println!("  1. Add API key to ~/.openat/config.json");
    println!("  2. Run: openat agent -m \"Hello!\"");
    Ok(())
}

fn create_template(path: &Path, name: &str, content: &str) -> Result<()> {
    let file = path.join(name);
    if !file.exists() {
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&file, content)?;
        println!("[+] Created {}", file.display());
    }
    Ok(())
}

/// Show status
pub fn status() -> Result<()> {
    println!("{}", LOGO);
    println!("\nopenat Status");
    println!("==============");

    let config_path = config::config_path();
    let _workspace = config::ensure_workspace_exists();

    println!("\nConfig: {}", config_path.display());

    let config = Config::load();
    println!("Model: {}", config.agents.defaults.model);

    let has_openrouter = !config.providers.openrouter.api_key.is_empty();
    let has_anthropic = !config.providers.anthropic.api_key.is_empty();
    let has_openai = !config.providers.openai.api_key.is_empty();

    println!("\nAPI Keys:");
    println!("  OpenRouter: {}", if has_openrouter { "[+] Set" } else { "[-] Not set" });
    println!("  Anthropic:  {}", if has_anthropic { "[+] Set" } else { "[-] Not set" });
    println!("  OpenAI:     {}", if has_openai { "[+] Set" } else { "[-] Not set" });

    println!("\nChannels:");
    println!("  Telegram: {}", if config.channels.telegram.enabled {
        "[+] Enabled"
    } else {
        "[-] Disabled"
    });
    println!("  WhatsApp: {}", if config.channels.whatsapp.enabled {
        "[+] Enabled"
    } else {
        "[-] Disabled"
    });

    Ok(())
}
