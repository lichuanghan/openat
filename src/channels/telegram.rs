use crate::bus::{InboundMessage, MessageBus};
use crate::config::Config;
use teloxide::Bot;
use teloxide::payloads::GetUpdatesSetters;
use teloxide::prelude::{Request, Requester};
use tracing::info;
use std::sync::Arc;
use tokio::time::{interval, Duration};

/// Telegram channel implementation
#[derive(Clone)]
pub struct TelegramChannel {
    bot: Bot,
    bus: MessageBus,
    allowed_users: Vec<String>,
    last_update_id: i32,
}

impl TelegramChannel {
    pub fn new(bot: Bot, bus: MessageBus, allowed_users: Vec<String>) -> Self {
        Self {
            bot,
            bus,
            allowed_users,
            last_update_id: 0,
        }
    }

    fn is_allowed(&self, user_id: &str) -> bool {
        self.allowed_users.is_empty() || self.allowed_users.contains(&user_id.to_string())
    }

    async fn fetch_updates(&mut self) -> anyhow::Result<()> {
        let updates = self.bot.get_updates()
            .offset(self.last_update_id + 1)
            .limit(100)
            .timeout(30)
            .send()
            .await?;

        for update in updates {
            // Update the last update id
            self.last_update_id = update.id.0 as i32;

            // Handle message updates
            if let teloxide::types::UpdateKind::Message(msg) = update.kind {
                self.handle_message(&msg).await;
            }
        }

        Ok(())
    }

    async fn handle_message(&self, msg: &teloxide::types::Message) {
        let chat_id = msg.chat.id.0.to_string();

        let sender_id = msg.from.as_ref()
            .map(|u| u.id.0.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Check if user is allowed
        if !self.is_allowed(&sender_id) {
            let _ = self.bot.send_message(msg.chat.id, "You are not authorized to use this bot.")
                .await;
            return;
        }

        let content = msg.text().unwrap_or("").to_string();

        if content.is_empty() {
            return;
        }

        info!("Received message from {} in chat {}: {}", sender_id, chat_id, content);

        // Create and publish inbound message
        let inbound = InboundMessage::new(
            "telegram",
            &sender_id,
            &chat_id,
            &content,
        );

        self.bus.publish_inbound(inbound).await;
    }

    /// Start the polling loop
    pub async fn run(&mut self) {
        info!("Starting Telegram polling...");

        let mut interval = interval(Duration::from_secs(1));

        loop {
            interval.tick().await;

            if let Err(e) = self.fetch_updates().await {
                tracing::error!("Error fetching updates: {}", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

/// Start the Telegram bot
pub async fn start_telegram_bot(config: &Config, bus: &MessageBus) -> anyhow::Result<TelegramChannel> {
    let token = &config.channels.telegram.token;

    if token.is_empty() {
        tracing::warn!("Telegram token not configured");
        return Err(anyhow::anyhow!("Telegram token not configured"));
    }

    let bot = Bot::new(token);

    // Get allowed users from config
    let allowed_users: Vec<String> = config.channels.telegram.allowed_users
        .iter()
        .cloned()
        .collect();

    let me = bot.get_me().await?;
    let username = me.username();
    let username_str = if username.is_empty() { "unknown" } else { username };
    info!("Logged in as @{}", username_str);

    let channel = Arc::new(TelegramChannel::new(bot.clone(), bus.clone(), allowed_users.clone()));

    // Start polling in background
    let mut channel_clone = (*channel).clone();
    tokio::spawn(async move {
        channel_clone.run().await;
    });

    Ok(TelegramChannel {
        bot,
        bus: bus.clone(),
        allowed_users,
        last_update_id: 0,
    })
}

/// Send a message through Telegram
pub async fn send_telegram_message(bot: &Bot, chat_id: i64, text: &str) -> anyhow::Result<()> {
    bot.send_message(teloxide::types::ChatId(chat_id), text)
        .await?;

    Ok(())
}
