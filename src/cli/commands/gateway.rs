//! Gateway command - starts the main bot gateway.

use crate::channels::ChannelManager;
use crate::config::Config;
use crate::core::agent::AgentExecutor;
use crate::core::scheduler::Scheduler;
use crate::core::MessageBus;
use crate::heartbeat::Heartbeat;
use crate::llm::create_provider;
use anyhow::Result;

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
    let mut agent_executor = AgentExecutor::new(provider, &config, &bus);

    // Create scheduler
    let scheduler = Scheduler::new(&bus);

    // Initialize channels
    let mut channel_manager = ChannelManager::new();
    channel_manager.initialize(&config).await?;

    println!("\n{}", LOGO);
    println!("Gateway components initialized:");
    println!("  [-] Heartbeat: running");
    println!("  [-] Agent Executor: ready");
    println!("  [-] Scheduler: ready");
    println!("  [-] Channels: initialized");

    // Run components concurrently
    let agent_task = tokio::spawn(async move {
        let mut inbound_rx = bus.subscribe_inbound();
        while let Ok(msg) = inbound_rx.recv().await {
            tracing::info!("Processing message from {}", msg.channel);
            if let Err(e) = agent_executor.handle_message(&msg).await {
                tracing::error!("Agent error: {}", e);
            }
        }
    });

    let scheduler_task = tokio::spawn(async move {
        scheduler.run().await;
    });

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
    channel_manager.stop().await;

    println!("Gateway stopped.");

    Ok(())
}
