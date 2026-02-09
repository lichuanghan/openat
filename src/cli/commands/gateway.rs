//! Gateway command - starts the main bot gateway.

use crate::channels::discord::DiscordChannel;
use crate::channels::Channel;
use crate::config::Config;
use crate::core::agent::AgentExecutor;
use crate::core::scheduler::Scheduler;
use crate::core::MessageBus;
use crate::heartbeat::Heartbeat;
use crate::llm::create_provider;
use anyhow::Result;
use tracing::info;

pub const LOGO: &str = r#"
        ()-()
      .-(___)-.
       _<   >_
       \/   \/
"#;

pub async fn execute(port: u16) -> Result<()> {
    println!("{}", LOGO);
    println!("Starting gateway on port {}...", port);

    // Load configuration
    let config = Config::load();

    // Create message bus for component communication
    let bus = MessageBus::new();

    // Start heartbeat
    let heartbeat = Heartbeat::new();
    heartbeat.start();

    // Create agent executor
    let provider = create_provider(&config);
    let agent_executor = AgentExecutor::new(provider, &config, &bus);

    // Create scheduler
    let scheduler = Scheduler::new(&bus);

    // Initialize Discord channel if enabled
    let mut discord_channel = None;
    if config.channels.discord.enabled && !config.channels.discord.token.is_empty() {
        info!("Initializing Discord channel...");
        let channel = DiscordChannel::new(config.channels.discord.clone());
        discord_channel = Some(channel);
    }

    println!("\n{}", LOGO);
    println!("Gateway components initialized:");
    println!("  [-] Heartbeat: running");
    println!("  [-] Agent Executor: ready");
    println!("  [-] Scheduler: ready");

    if discord_channel.is_some() {
        println!("  [-] Discord: starting...");
    }

    // Run components concurrently
    let bus_for_agent = bus.clone();
    let agent_task = tokio::spawn(async move {
        let mut executor = agent_executor;
        let mut inbound_rx = bus_for_agent.subscribe_inbound();
        while let Ok(msg) = inbound_rx.recv().await {
            tracing::info!("Processing message from {}", msg.channel);
            if let Err(e) = executor.handle_message(&msg).await {
                tracing::error!("Agent error: {}", e);
            }
        }
    });

    let scheduler_task = tokio::spawn(async move {
        scheduler.run().await;
    });

    // Start Discord channel if enabled
    if let Some(ref mut channel) = discord_channel {
        if let Err(e) = channel.start(&bus).await {
            tracing::error!("Failed to start Discord channel: {}", e);
        } else {
            println!("  [-] Discord: connected!");
        }
    }

    println!("\nGateway running. Press Ctrl+C to stop.");
    println!("Heartbeat: {}", heartbeat.uptime());

    // Wait for shutdown signal
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("\nShutting down gateway...");
        }
        _ = agent_task => {
            println!("Agent task ended unexpectedly");
        }
        _ = scheduler_task => {
            println!("Scheduler task ended unexpectedly");
        }
    }

    // Cleanup
    heartbeat.stop();

    // Stop Discord channel
    if let Some(ref mut channel) = discord_channel {
        let _ = channel.stop().await;
    }

    println!("Gateway stopped.");

    Ok(())
}
