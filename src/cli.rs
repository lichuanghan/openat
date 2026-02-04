use crate::agent::Agent;
use crate::config::{self, Config};
use crate::cron::{CronJob, CronManager};
use crate::heartbeat::Heartbeat;
use crate::providers::create_provider;
use anyhow::Result;
use std::io::{self, Write};

pub const LOGO: &str = r#"
    |\__/,|   (`\
  _.|o o  |_   ) )
 -(((---(((--------
"#;

pub async fn onboard() -> Result<()> {
    println!("{}", LOGO);
    println!("Initializing nanobot...\n");

    let workspace = config::ensure_workspace_exists();

    let config = Config::load();
    config.save()?;
    println!("[+] Created ~/.nanobot/config.json");

    create_template(&workspace, "AGENTS.md", "# Agent Instructions\n\nYou are a helpful AI assistant.")?;
    create_template(&workspace, "SOUL.md", "# Soul\n\nI am nanobot, an AI assistant.")?;
    create_template(&workspace, "USER.md", "# User\n\nInformation about the user.")?;
    create_template(&workspace.join("memory"), "MEMORY.md", "# Memory\n\nImportant information.")?;

    println!("[+] Created workspace at ~/.nanobot/workspace");
    println!("\n{}", LOGO);
    println!("Ready! Next steps:");
    println!("  1. Add API key to ~/.nanobot/config.json");
    println!("  2. Run: nanobot agent -m \"Hello!\"");
    Ok(())
}

fn create_template(path: &std::path::Path, name: &str, content: &str) -> Result<()> {
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

pub async fn gateway(port: u16) -> Result<()> {
    println!("{}", LOGO);
    println!("Starting gateway on port {}...", port);

    // Start heartbeat
    let heartbeat = Heartbeat::new();
    heartbeat.start();

    // TODO: Start channel manager with Telegram/WhatsApp
    println!("Gateway running. Press Ctrl+C to stop.");
    println!("Heartbeat: {}", heartbeat.uptime());

    // Keep running
    tokio::signal::ctrl_c().await?;
    heartbeat.stop();
    println!("Gateway stopped.");

    Ok(())
}

pub async fn agent(message: &str) -> Result<()> {
    println!("{}", LOGO);

    let config = Config::load();
    let workspace = config::ensure_workspace_exists();

    let provider = create_provider(&config);
    let agent = Agent::new(
        provider,
        config.agents.defaults.model.clone(),
        workspace,
    );

    println!("\nYou: {}", message);
    print!("Agent: ");
    io::stdout().flush()?;

    let response = agent.chat(message).await;
    println!("{}", response);
    Ok(())
}

pub async fn interactive_agent() -> Result<()> {
    println!("{}", LOGO);
    println!("\nInteractive mode (Ctrl+C to exit)\n");

    let config = Config::load();
    let workspace = config::ensure_workspace_exists();

    if config.get_api_key().is_none() {
        println!("Warning: No API key configured. Add to ~/.nanobot/config.json");
    }

    let provider = create_provider(&config);

    let agent = Agent::new(
        provider,
        config.agents.defaults.model.clone(),
        workspace,
    );

    loop {
        print!("You: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        print!("Agent: ");
        io::stdout().flush()?;

        let response = agent.chat(input).await;
        println!("{}\n", response);
    }
}

pub fn status() -> Result<()> {
    println!("{}", LOGO);
    println!("\nnanobot Status");
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

pub fn channel_status() -> Result<()> {
    println!("=== Channel Status ===");
    println!("{}", "=".repeat(50));

    let config = Config::load();

    println!("Telegram: {}", if config.channels.telegram.enabled {
        "[+] Enabled"
    } else {
        "[-] Disabled"
    });
    if !config.channels.telegram.token.is_empty() {
        println!("  Token: {}...", &config.channels.telegram.token[..10]);
    }

    println!("WhatsApp: {}", if config.channels.whatsapp.enabled {
        "[+] Enabled"
    } else {
        "[-] Disabled"
    });
    println!("  Bridge: {}", config.channels.whatsapp.bridge_url);

    Ok(())
}

pub async fn channel_login(channel: Option<&str>) -> Result<()> {
    let channel = channel.unwrap_or("telegram");

    match channel {
        "telegram" => {
            println!("Telegram login not yet implemented.");
            println!("Add your bot token to ~/.nanobot/config.json");
        }
        "whatsapp" => {
            println!("WhatsApp login: Scan the QR code from WA Bridge");
        }
        _ => {
            println!("Unknown channel: {}", channel);
        }
    }

    Ok(())
}

pub fn cron_list(all: bool) -> Result<()> {
    println!("=== Cron Jobs ===");
    println!("{}", "=".repeat(50));

    let jobs_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".nanobot")
        .join("cron");

    let manager = CronManager::new(jobs_dir);
    let jobs = manager.load_jobs();

    if jobs.is_empty() {
        println!("No scheduled jobs.");
        return Ok(());
    }

    for job in jobs {
        if !all && !job.enabled {
            continue;
        }
        println!("\n[{}] {}", if job.enabled { "X" } else { " " }, job.name);
        println!("  ID: {}", job.id);
        println!("  Message: {}", job.message);
        if let Some(interval) = job.interval_seconds {
            println!("  Every: {} seconds", interval);
        }
        if let Some(next) = job.next_run {
            println!("  Next run: {}", next);
        }
    }

    Ok(())
}

pub fn cron_add(
    name: &str,
    message: &str,
    every: Option<u64>,
    cron: Option<String>,
    deliver: bool,
    to: Option<&str>,
    channel: Option<&str>,
) -> Result<()> {
    let jobs_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".nanobot")
        .join("cron");

    let mut manager = CronManager::new(jobs_dir);

    let mut job = CronJob::new(name.to_string(), message.to_string());
    job.interval_seconds = every;
    job.cron_expression = cron;
    job.deliver_response = deliver;
    job.deliver_to = to.map(|s| s.to_string());
    job.deliver_channel = channel.map(|s| s.to_string());

    manager.add_job(&mut job);

    println!("[+] Created cron job: {}", name);
    if let Some(next) = job.next_run {
        println!("  Next run: {}", next);
    }

    Ok(())
}

pub fn cron_remove(job_id: &str) -> Result<()> {
    let jobs_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".nanobot")
        .join("cron");

    let manager = CronManager::new(jobs_dir);

    if manager.delete_job(job_id) {
        println!("[+] Removed cron job: {}", job_id);
    } else {
        println!("[-] Cron job not found: {}", job_id);
    }

    Ok(())
}

pub fn cron_enable(job_id: &str, disable: bool) -> Result<()> {
    let jobs_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".nanobot")
        .join("cron");

    let mut manager = CronManager::new(jobs_dir);

    if manager.toggle_job(job_id, !disable) {
        println!("[{}] Cron job: {}", if disable { "Disabled" } else { "Enabled" }, job_id);
    } else {
        println!("[-] Cron job not found: {}", job_id);
    }

    Ok(())
}
