use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::layer::SubscriberExt;

const VERSION: &str = "0.1.0";

/// Initialize tracing with environment-based filtering
fn init_logging() {
    // Try to get RUST_LOG from environment, default to info
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
    /// Show status
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();
    tracing::info!(version = VERSION, "Starting openat");
    let args = Args::parse();

    match args.command {
        Commands::Onboard => cli::onboard().await?,
        Commands::Gateway { port } => cli::gateway(port.unwrap_or(18790)).await?,
        Commands::Agent { message } => {
            if let Some(msg) = message {
                cli::agent(&msg).await?
            } else {
                cli::agent_interactive().await?
            }
        }
        Commands::ChannelStatus => cli::channel_status()?,
        Commands::ChannelLogin { channel } => cli::channel_login(channel.as_deref()).await?,
        Commands::CronList { all } => cli::cron_list(all)?,
        Commands::CronAdd { name, message, every, cron, deliver, to, channel } => {
            cli::cron_add(&name, &message, every, cron, deliver, to.as_deref(), channel.as_deref())?
        }
        Commands::CronRemove { job_id } => cli::cron_remove(&job_id)?,
        Commands::CronEnable { job_id, disable } => cli::cron_enable(&job_id, disable)?,
        Commands::Status => cli::status()?,
    }

    Ok(())
}

mod channels;
mod cli;
mod config;
mod core;
mod heartbeat;
mod llm;
mod tools;
mod types;
