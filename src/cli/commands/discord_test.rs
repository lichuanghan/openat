//! Discord test command - sends a test message to Discord.

use crate::config::Config;
use anyhow::Result;
use serde_json::json;

/// Send a test message to a Discord channel
pub async fn execute(channel_id: &str, content: &str) -> Result<()> {
    let config = Config::load();

    if config.channels.discord.token.is_empty() {
        anyhow::bail!("Discord token not configured. Add it to ~/.openat/config.json");
    }

    println!("Sending message to Discord channel {}...", channel_id);

    let client = reqwest::Client::new();
    let response = client
        .post(&format!(
            "https://discord.com/api/v10/channels/{}/messages",
            channel_id
        ))
        .header("Authorization", format!("Bot {}", config.channels.discord.token))
        .header("Content-Type", "application/json")
        .json(&json!({ "content": content }))
        .send()
        .await?;

    if response.status().is_success() {
        println!("[+] Message sent successfully!");
        let text = response.text().await?;
        println!("Response: {}", text);
    } else {
        let error = response.text().await?;
        anyhow::bail!("Failed to send message: {}", error);
    }

    Ok(())
}
