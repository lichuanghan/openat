//! openat CLI entry point
//!
//! Multi-channel AI bot with LLM capabilities

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::layer::SubscriberExt;
use std::sync::Arc;

const VERSION: &str = "0.1.0";

/// Initialize tracing with environment-based filtering
fn init_logging() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("openat=info"));

    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(env_filter);

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");
}

#[derive(Parser, Debug)]
#[command(name = "openat")]
#[command(version = VERSION)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize configuration and workspace
    Onboard,
    /// Start the gateway
    Gateway { port: Option<u16> },
    /// Chat with the agent
    Agent { message: Option<String> },
    /// Show channel status
    ChannelStatus,
    /// Login/link a channel
    ChannelLogin { channel: Option<String> },
    /// List scheduled jobs
    CronList { all: bool },
    /// Add a scheduled job
    CronAdd {
        name: String,
        message: String,
        every: Option<u64>,
        cron: Option<String>,
        deliver: bool,
        to: Option<String>,
        channel: Option<String>,
    },
    /// Remove a job
    CronRemove { job_id: String },
    /// Enable/disable a job
    CronEnable { job_id: String, disable: bool },
    /// Test Discord (send a message)
    DiscordTest {
        channel_id: String,
        message: Option<String>,
    },
    /// Show status
    Status,
    /// Initialize default config
    Init,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();
    tracing::info!(version = VERSION, "Starting openat");
    let args = Args::parse();

    match args.command {
        Commands::Onboard => onboard().await?,
        Commands::Gateway { port } => gateway(port.unwrap_or(18790)).await?,
        Commands::Agent { message } => {
            if let Some(msg) = message {
                agent(&msg).await?
            } else {
                agent_interactive().await?
            }
        }
        Commands::ChannelStatus => channel_status()?,
        Commands::ChannelLogin { channel } => channel_login(channel.as_deref()).await?,
        Commands::CronList { all } => cron_list(all)?,
        Commands::CronAdd { name, message, every, cron, deliver, to, channel } => {
            cron_add(&name, &message, every, cron.as_deref(), deliver, to.as_deref(), channel.as_deref())?
        }
        Commands::CronRemove { job_id } => cron_remove(&job_id)?,
        Commands::CronEnable { job_id, disable } => cron_enable(&job_id, disable)?,
        Commands::DiscordTest { channel_id, message } => {
            let content = message.unwrap_or_else(|| "Test message from openat!".to_string());
            discord_test(&channel_id, &content).await?
        }
        Commands::Status => status()?,
        Commands::Init => init_config()?,
    }

    Ok(())
}

/// Initialize default configuration
fn init_config() -> Result<()> {
    let config = openat_config::Config::default();
    config.save()?;
    tracing::info!("Created default config at ~/.openat/config.json");
    Ok(())
}

/// Initialize configuration and workspace
async fn onboard() -> Result<()> {
    tracing::info!("Onboarding...");
    // Create default config
    let config = openat_config::Config::default();
    config.save()?;
    tracing::info!("Created default config at ~/.openat/config.json");

    // Ensure workspace directory exists
    let _ = openat_config::ensure_workspace_exists();
    tracing::info!("Workspace initialized at ~/.openat/workspace");
    Ok(())
}

/// Start the gateway
async fn gateway(port: u16) -> Result<()> {
    use std::net::SocketAddr;

    tracing::info!("Gateway starting on port {}", port);

    // Load configuration
    let config = openat_config::Config::load();
    let provider = create_provider(&config)?;
    let bus = openat_runtime::MessageBus::new();
    let executor = Arc::new(tokio::sync::Mutex::new(openat_agent::AgentExecutor::new(provider, &config, &bus)));

    // Start Discord channel if enabled
    if config.channels.discord.enabled {
        let mut discord = openat_channels::DiscordChannel::new(config.channels.discord.clone());
        let bus_clone = bus.clone();
        tokio::spawn(async move {
            use openat_channels::Channel;
            if let Err(e) = discord.start(&bus_clone).await {
                tracing::error!("Discord channel error: {}", e);
            }
        });
        tracing::info!("Discord channel started");
    }

    // Start QQ channel if enabled
    if config.channels.qq.enabled {
        let mut qq = openat_channels::QQChannel::new(config.channels.qq.clone());
        let bus_clone = bus.clone();
        tokio::spawn(async move {
            use openat_channels::Channel;
            if let Err(e) = qq.start(&bus_clone).await {
                tracing::error!("QQ channel error: {}", e);
            }
        });
        tracing::info!("QQ channel started");
    }

    // Start inbound message handler (connects bus -> agent -> bus)
    {
        let executor = executor.clone();
        let mut inbound_rx = bus.subscribe_inbound();
        tokio::spawn(async move {
            loop {
                match inbound_rx.recv().await {
                    Ok(msg) => {
                        tracing::info!("Processing inbound message from {}: {}", msg.channel, msg.content);
                        let mut exec = executor.lock().await;
                        match exec.handle_message(&msg).await {
                            Ok(resp) => {
                                tracing::info!("Agent response: {}", resp.content);
                            }
                            Err(e) => {
                                tracing::error!("Agent error: {}", e);
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Inbound receiver lagged by {} messages", n);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        });
    }

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Gateway running at http://0.0.0.0:{}/", port);

    loop {
        let (stream, _) = listener.accept().await?;
        let executor = executor.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, executor).await {
                tracing::error!("Connection error: {}", e);
            }
        });
    }
}

async fn handle_connection(
    mut stream: tokio::net::TcpStream,
    executor: Arc<tokio::sync::Mutex<openat_agent::AgentExecutor>>,
) -> Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut buffer = [0u8; 4096];
    let bytes_read = stream.read(&mut buffer).await?;
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);

    // Simple HTTP parser
    let response: String = if request.contains("GET /health") {
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\n\r\nOK".to_string()
    } else if request.contains("POST /chat") {
        // Extract JSON body
        if let Some(body_start) = request.find("{") {
            let body = &request[body_start..];
            match serde_json::from_str::<ChatRequest>(body) {
                Ok(req) => {
                    let inbound = openat_types::InboundMessage::new(
                        &req.channel,
                        &req.user,
                        &req.chat_id,
                        &req.message,
                    );
                    let mut exec = executor.lock().await;
                    match exec.handle_message(&inbound).await {
                        Ok(resp) => {
                            let chat_resp = ChatResponse {
                                success: true,
                                response: resp.content,
                            };
                            let json = serde_json::to_string(&chat_resp).unwrap_or_default();
                            format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", json.len(), json)
                        }
                        Err(e) => {
                            let chat_resp = ChatResponse {
                                success: false,
                                response: format!("Error: {}", e),
                            };
                            let json = serde_json::to_string(&chat_resp).unwrap_or_default();
                            format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", json.len(), json)
                        }
                    }
                }
                Err(e) => {
                    let err_msg = format!("Error: {}", e);
                    format!("HTTP/1.1 400 Bad Request\r\nContent-Length: {}\r\n\r\n{}", err_msg.len(), err_msg)
                }
            }
        } else {
            "HTTP/1.1 400 Bad Request\r\nContent-Length: 17\r\n\r\nMissing JSON body".to_string()
        }
    } else if request.contains("GET /") {
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html><body><h1>OpenAT Gateway</h1><p>POST /chat to send messages</p></body></html>".to_string()
    } else {
        "HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\n\r\nNot Found".to_string()
    };

    stream.write_all(response.as_bytes()).await?;
    Ok(())
}

/// Chat request structure
#[derive(serde::Deserialize)]
struct ChatRequest {
    message: String,
    channel: String,
    user: String,
    chat_id: String,
}

/// Chat response structure
#[derive(serde::Serialize)]
struct ChatResponse {
    success: bool,
    response: String,
}

/// Chat with the agent
async fn agent(message: &str) -> Result<()> {
    tracing::info!("Agent message: {}", message);

    // Load configuration
    let config = openat_config::Config::load();

    // Create provider
    let provider = create_provider(&config)?;

    // Create message bus
    let bus = openat_runtime::MessageBus::new();

    // Create agent executor
    let mut executor = openat_agent::AgentExecutor::new(provider, &config, &bus);

    // Create a test inbound message
    let inbound = openat_types::InboundMessage::new("cli", "user", "cli", message);

    // Handle the message
    match executor.handle_message(&inbound).await {
        Ok(response) => {
            println!("\nAssistant: {}", response.content);
        }
        Err(e) => {
            tracing::error!("Agent error: {}", e);
            println!("Error: {}", e);
        }
    }

    Ok(())
}

/// Interactive agent chat
async fn agent_interactive() -> Result<()> {
    tracing::info!("Starting interactive agent mode...");
    println!("Interactive mode (press Ctrl+C to exit)");

    // Load configuration
    let config = openat_config::Config::load();

    // Create provider
    let provider = create_provider(&config)?;

    // Create message bus
    let bus = openat_runtime::MessageBus::new();

    // Create agent executor
    let mut executor = openat_agent::AgentExecutor::new(provider, &config, &bus);

    println!("Type your message and press Enter. Ctrl+C to exit.\n");

    // Simple loop for input
    use std::io::{self, BufRead};

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(input) if !input.trim().is_empty() => {
                let inbound = openat_types::InboundMessage::new("cli", "user", "cli", &input);
                match executor.handle_message(&inbound).await {
                    Ok(response) => {
                        println!("\nAssistant: {}\n", response.content);
                    }
                    Err(e) => {
                        println!("\nError: {}\n", e);
                    }
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }

    Ok(())
}

/// Show channel status
fn channel_status() -> Result<()> {
    tracing::info!("Showing channel status");
    // Load config and show enabled channels
    let config = openat_config::Config::load();
    println!("Channel Status:");
    println!("  Discord:  {}", if config.channels.discord.enabled { "enabled" } else { "disabled" });
    println!("  Telegram: {}", if config.channels.telegram.enabled { "enabled" } else { "disabled" });
    Ok(())
}

/// Login/link a channel
async fn channel_login(channel: Option<&str>) -> Result<()> {
    let channel = channel.unwrap_or("discord");
    tracing::info!("Logging into channel: {}", channel);
    println!("Channel login for {} (feature not yet implemented)", channel);
    Ok(())
}

/// List scheduled jobs
fn cron_list(all: bool) -> Result<()> {
    tracing::info!("Listing cron jobs (all: {})", all);
    println!("Cron jobs (feature not yet implemented)");
    Ok(())
}

/// Add a scheduled job
fn cron_add(
    name: &str,
    message: &str,
    every: Option<u64>,
    cron: Option<&str>,
    deliver: bool,
    to: Option<&str>,
    channel: Option<&str>,
) -> Result<()> {
    tracing::info!(
        "Adding cron job: name={}, message={}, every={:?}, cron={:?}, deliver={}, to={:?}, channel={:?}",
        name, message, every, cron, deliver, to, channel
    );
    println!("Cron job added (feature not yet implemented)");
    Ok(())
}

/// Remove a job
fn cron_remove(job_id: &str) -> Result<()> {
    tracing::info!("Removing cron job: {}", job_id);
    println!("Cron job removed (feature not yet implemented)");
    Ok(())
}

/// Enable/disable a job
fn cron_enable(job_id: &str, disable: bool) -> Result<()> {
    let action = if disable { "Disabling" } else { "Enabling" };
    let state = if disable { "disabled" } else { "enabled" };
    tracing::info!("{} cron job: {}", action, job_id);
    println!("Cron job {} (feature not yet implemented)", state);
    Ok(())
}

/// Test Discord
async fn discord_test(channel_id: &str, message: &str) -> Result<()> {
    tracing::info!("Discord test: channel={}, message={}", channel_id, message);
    println!("Discord test sent (feature not yet implemented)");
    Ok(())
}

/// Show status
fn status() -> Result<()> {
    tracing::info!("Showing status");
    let config = openat_config::Config::load();
    println!("OpenAT Status:");
    println!("  Version: {}", VERSION);
    println!("  Discord:  {}", if config.channels.discord.enabled { "enabled" } else { "disabled" });
    println!("  Telegram: {}", if config.channels.telegram.enabled { "enabled" } else { "disabled" });

    // Check for API keys
    let has_key = config.get_api_key().is_some();
    println!("  LLM:      {}", if has_key { "configured" } else { "not configured" });
    Ok(())
}

/// Create LLM provider based on configuration
fn create_provider(config: &openat_config::Config) -> Result<Arc<dyn openat_providers::LLMProvider>> {
    // Priority: OpenRouter > Anthropic > OpenAI > Groq > Gemini > MiniMax
    if !config.providers.openrouter.api_key.is_empty() {
        let provider = openat_providers::OpenRouterProvider::new(config.providers.openrouter.api_key.clone());
        return Ok(Arc::new(provider));
    }

    if !config.providers.anthropic.api_key.is_empty() {
        let provider = openat_providers::AnthropicProvider::new(config.providers.anthropic.api_key.clone());
        return Ok(Arc::new(provider));
    }

    if !config.providers.openai.api_key.is_empty() {
        let provider = openat_providers::OpenAICompatProvider::new(
            openat_providers::OpenAICompatConfig::new(
                config.providers.openai.api_key.clone(),
                config.providers.openai.api_base.clone().unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
                "openai",
            )
        );
        return Ok(Arc::new(provider));
    }

    if !config.providers.groq.api_key.is_empty() {
        let provider = openat_providers::GroqProvider::new(config.providers.groq.api_key.clone());
        return Ok(Arc::new(provider));
    }

    if !config.providers.gemini.api_key.is_empty() {
        let provider = openat_providers::GeminiProvider::new(config.providers.gemini.api_key.clone());
        return Ok(Arc::new(provider));
    }

    if !config.providers.minimax.api_key.is_empty() {
        let provider = openat_providers::MiniMaxProvider::new(
            config.providers.minimax.api_key.clone(),
            config.providers.minimax.api_base.clone(),
        );
        return Ok(Arc::new(provider));
    }

    anyhow::bail!("No API key configured. Please set one in ~/.openat/config.json");
}
