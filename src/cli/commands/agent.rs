//! Agent command - chat with the agent.

use crate::config::{self, Config};
use crate::core::agent::SimpleAgent;
use crate::llm::create_provider;
use anyhow::Result;
use std::io::{self, Write};

pub const LOGO: &str = r#"
    |\__/,|   (`\
  _.|o o  |_   ) )
 -(((---(((--------
"#;

/// Execute a single message with the agent
pub async fn execute(message: &str) -> Result<()> {
    println!("{}", LOGO);

    let config = Config::load();
    let workspace = config::ensure_workspace_exists();

    let provider = create_provider(&config);
    let agent = SimpleAgent::new(
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

/// Run interactive chat mode
pub async fn interactive() -> Result<()> {
    println!("{}", LOGO);
    println!("\nInteractive mode (Ctrl+C to exit)\n");

    let config = Config::load();
    let workspace = config::ensure_workspace_exists();

    if config.get_api_key().is_none() {
        println!("Warning: No API key configured. Add to ~/.openat/config.json");
    }

    let provider = create_provider(&config);

    let agent = SimpleAgent::new(
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
