//! openat CLI entry point
//!
//! Multi-channel AI bot with LLM capabilities

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter};
use std::sync::Arc;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::UNIX_EPOCH;

const VERSION: &str = "0.1.0";

/// Global start time (seconds since UNIX_EPOCH)
static START_TIME: AtomicU64 = AtomicU64::new(0);

/// Set the start time (call once at startup)
fn set_start_time() {
    let now = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    START_TIME.store(now, Ordering::SeqCst);
}

/// Get uptime in seconds
fn get_uptime() -> u64 {
    let start = START_TIME.load(Ordering::SeqCst);
    if start == 0 {
        return 0;
    }
    let now = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    now.saturating_sub(start)
}

/// Simple FNV hash function for deduplication
fn fnv_hash(s: &str) -> u64 {
    const FNV_PRIME: u64 = 1099511628211;
    const FNV_OFFSET: u64 = 14695981039346656037;
    let mut hash = FNV_OFFSET;
    for byte in s.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Get log directory path
fn get_log_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".openat/logs")
    } else {
        PathBuf::from("logs")
    }
}

/// Initialize tracing with environment-based filtering and file output
fn init_logging() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("openat=info"));

    // Create log directory
    let log_dir = get_log_dir();
    let _ = std::fs::create_dir_all(&log_dir);

    let file_appender = tracing_appender::rolling::daily(&log_dir, "openat.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Keep the guard alive for the duration of the program
    std::mem::forget(_guard);

    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::new("openat=debug"))
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(true)
                .with_target(false)
                .compact()
        )
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
    Agent {
        message: Option<String>,
        /// Enable streaming response
        #[arg(short, long, default_value = "false")]
        stream: bool
    },
    /// Test streaming response
    TestStream { message: String },
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
        Commands::Agent { message, stream } => {
            if let Some(msg) = message {
                if stream {
                    agent_stream(&msg).await?
                } else {
                    agent(&msg).await?
                }
            } else {
                if stream {
                    agent_interactive_stream().await?
                } else {
                    agent_interactive().await?
                }
            }
        }
        Commands::TestStream { message } => test_stream(&message).await?,
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

    // Set start time for uptime tracking
    set_start_time();

    tracing::info!("Gateway starting on port {}", port);

    // Load configuration
    let config = openat_config::Config::load();
    let provider = create_provider(&config)?;
    let bus = openat_runtime::MessageBus::new();
    let executor = Arc::new(tokio::sync::Mutex::new(openat_agent::AgentExecutor::new(provider, &config, &bus)));

    // Initialize skills (load from workspace)
    {
        let mut exec = executor.lock().await;
        exec.init_skills().await;
    }

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
        use std::collections::HashSet;
        use std::sync::Mutex;
        use std::time::{Duration, Instant};

        let executor = executor.clone();
        let mut inbound_rx = bus.subscribe_inbound();

        // Deduplication cache: message_hash -> timestamp
        let dedup_cache: Arc<Mutex<HashSet<(u64, Instant)>>> = Arc::new(Mutex::new(HashSet::new()));
        let dedup_clone = dedup_cache.clone();

        // Cleanup task for dedup cache
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                if let Ok(mut cache) = dedup_clone.lock() {
                    let now = Instant::now();
                    cache.retain(|(_, t)| now.duration_since(*t) < Duration::from_secs(60));
                }
            }
        });

        tokio::spawn(async move {
            loop {
                match inbound_rx.recv().await {
                    Ok(msg) => {
                        // Create a simple hash from channel + sender_id + content
                        let msg_hash = fnv_hash(&format!("{}:{}:{}",
                            msg.channel, msg.sender_id, msg.content));

                        // Check for duplicates (within last 60 seconds)
                        let is_duplicate = {
                            if let Ok(cache) = dedup_cache.lock() {
                                cache.iter().any(|(h, t)| {
                                    *h == msg_hash && Instant::now().duration_since(*t) < Duration::from_secs(60)
                                })
                            } else {
                                false
                            }
                        };

                        if is_duplicate {
                            tracing::debug!("Duplicate message ignored: {}", msg.content);
                            continue;
                        }

                        // Add to cache
                        if let Ok(mut cache) = dedup_cache.lock() {
                            cache.insert((msg_hash, Instant::now()));
                        }

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

    // Create shutdown signal handler
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();

    // Spawn Ctrl+C handler
    tokio::spawn(async move {
        use tokio::signal;
        signal::ctrl_c().await.ok();
        tracing::info!("Received shutdown signal (Ctrl+C)");
        shutdown_clone.store(true, Ordering::SeqCst);
    });

    // Main accept loop with graceful shutdown
    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _)) => {
                        let executor = executor.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream, executor).await {
                                tracing::error!("Connection error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("Accept error: {}", e);
                    }
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                // Check for shutdown
                if shutdown.load(Ordering::SeqCst) {
                    tracing::info!("Shutting down gracefully...");
                    break;
                }
            }
        }
    }

    // Give some time for pending tasks to complete
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    tracing::info!("Gateway stopped");

    Ok(())
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
        // Enhanced health check with JSON response
        let health = serde_json::json!({
            "status": "healthy",
            "version": VERSION,
            "uptime_seconds": get_uptime(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        let json = health.to_string();
        format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", json.len(), json)
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
/// Test streaming response from the agent
async fn test_stream(message: &str) -> Result<()> {
    agent_stream(message).await
}

/// Get agent name prefix for display
fn agent_name_prefix(config: &openat_config::Config) -> String {
    let name = &config.agents.defaults.name;
    if name.is_empty() {
        "Assistant".to_string()
    } else {
        name.clone()
    }
}

/// Stream agent response
async fn agent_stream(message: &str) -> Result<()> {
    use futures_util::StreamExt;
    tracing::info!("Agent streaming message: {}", message);

    // Load configuration
    let config = openat_config::Config::load();

    // Create provider
    let provider = create_provider(&config)?;

    // Check if provider supports streaming
    if !provider.supports_streaming() {
        println!("Provider does not support streaming, using regular response");
        return agent(message).await;
    }

    // Create message bus
    let bus = openat_runtime::MessageBus::new();

    // Create agent executor
    let executor = openat_agent::AgentExecutor::new(provider, &config, &bus);

    // Initialize skills (load from workspace)
    executor.init_skills().await;

    // Create a test inbound message
    let inbound = openat_types::InboundMessage::new("cli", "user", "cli", message);

    // Handle the message with streaming
    let mut stream = executor.handle_message_streaming(&inbound);

    let agent_name = &config.agents.defaults.name;
    if agent_name.is_empty() {
        print!("\nAssistant: ");
    } else {
        print!("\n{}: ", agent_name);
    }
    std::io::Write::flush(&mut std::io::stdout()).unwrap();

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                print!("{}", chunk.content);
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
                if chunk.is_final {
                    println!("\n[Stream complete]");
                }
            }
            Err(e) => {
                println!("\nError: {}", e);
                return Err(anyhow::anyhow!(e));
            }
        }
    }

    Ok(())
}

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

    // Initialize skills (load from workspace)
    executor.init_skills().await;

    // Create a test inbound message
    let inbound = openat_types::InboundMessage::new("cli", "user", "cli", message);

    // Handle the message
    let name = agent_name_prefix(&config);
    match executor.handle_message(&inbound).await {
        Ok(response) => {
            println!("\n{}: {}", name, response.content);
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

    // Load configuration
    let config = openat_config::Config::load();
    let name = agent_name_prefix(&config);
    println!("{}: Interactive mode (press Ctrl+C to exit)", name);

    // Create provider
    let provider = create_provider(&config)?;

    // Create message bus
    let bus = openat_runtime::MessageBus::new();

    // Create agent executor
    let mut executor = openat_agent::AgentExecutor::new(provider, &config, &bus);

    // Initialize skills (load from workspace)
    executor.init_skills().await;

    println!("输入你的消息，回车发送。Ctrl+C 退出。\n");

    // Simple loop for input
    use std::io::{self, Write};

    let stdin = io::stdin();
    loop {
        print!("你: ");
        io::stdout().flush()?;
        let mut line = String::new();
        if stdin.read_line(&mut line).is_err() {
            break;
        }
        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        let inbound = openat_types::InboundMessage::new("cli", "user", "cli", input);
        match executor.handle_message(&inbound).await {
            Ok(response) => {
                println!("\n{}: {}\n", name, response.content);
            }
            Err(e) => {
                println!("\nError: {}\n", e);
            }
        }
    }

    Ok(())
}

/// Interactive agent chat with streaming
async fn agent_interactive_stream() -> Result<()> {
    use futures_util::StreamExt;
    tracing::info!("Starting interactive agent mode with streaming...");

    // Load configuration
    let config = openat_config::Config::load();
    let name = agent_name_prefix(&config);
    println!("{}: Interactive streaming mode (press Ctrl+C to exit)", name);

    // Create provider
    let provider = create_provider(&config)?;

    // Check if provider supports streaming
    let streaming_supported = provider.supports_streaming();
    if !streaming_supported {
        println!("Warning: Provider does not support streaming, falling back to regular mode");
    }

    // Create message bus
    let bus = openat_runtime::MessageBus::new();

    // Create agent executor
    let executor = openat_agent::AgentExecutor::new(provider, &config, &bus);

    // Initialize skills (load from workspace)
    executor.init_skills().await;

    println!("输入你的消息，回车发送。Ctrl+C 退出。\n");

    // Simple loop for input
    use std::io::{self, Write};

    let stdin = io::stdin();
    loop {
        print!("你: ");
        io::stdout().flush()?;
        let mut line = String::new();
        if stdin.read_line(&mut line).is_err() {
            break;
        }
        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        let inbound = openat_types::InboundMessage::new("cli", "user", "cli", input);

        if streaming_supported {
            // Streaming mode
            let mut stream = executor.handle_message_streaming(&inbound);
            print!("\n{}: ", name);
            io::Write::flush(&mut io::stdout())?;

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        print!("{}", chunk.content);
                        io::Write::flush(&mut io::stdout())?;
                        if chunk.is_final {
                            println!("\n");
                        }
                    }
                    Err(e) => {
                        println!("\nError: {}\n", e);
                    }
                }
            }
        } else {
            // Fallback to regular mode
            let mut exec = openat_agent::AgentExecutor::new(
                create_provider(&config)?, &config, &bus.clone()
            );
            match exec.handle_message(&inbound).await {
                Ok(response) => {
                    println!("\n{}: {}\n", name, response.content);
                }
                Err(e) => {
                    println!("\nError: {}\n", e);
                }
            }
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
