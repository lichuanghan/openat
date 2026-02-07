//! Channel commands - manage communication channels.

use crate::config::Config;
use anyhow::Result;

/// Show channel status
pub fn status() -> Result<()> {
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

    println!("QQ: {}", if config.channels.qq.enabled {
        "[+] Enabled"
    } else {
        "[-] Disabled"
    });
    println!("  Event URL: {}", config.channels.qq.event_url);
    if !config.channels.qq.access_token.is_empty() {
        println!("  Access Token: [+] Set");
    }

    Ok(())
}

/// Login to a channel
pub async fn login(channel: Option<&str>) -> Result<()> {
    let channel = channel.unwrap_or("telegram");

    match channel {
        "telegram" => {
            println!("Telegram login not yet implemented.");
            println!("Add your bot token to ~/.openat/config.json");
        }
        "whatsapp" => {
            println!("WhatsApp login: Scan the QR code from WA Bridge");
        }
        "qq" => {
            println!("QQ login via OneBot:");
            println!("1. Install go-cqhttp or another OneBot v11 implementation");
            println!("2. Configure it to connect to WebSocket: ws://localhost:3000");
            println!("3. Set the event_url in ~/.openat/config.json");
        }
        _ => {
            println!("Unknown channel: {}", channel);
        }
    }

    Ok(())
}
