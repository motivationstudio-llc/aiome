/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use super::bridge_trait::ChannelBridge;
use aiome_core::error::AiomeError;
use async_trait::async_trait;
use serenity::{
    all::GatewayIntents, all::Http, model::channel::Message as DiscordMessage,
    model::gateway::Ready, prelude::*,
};
use shared::watchtower::ControlCommand;
use std::sync::Arc;
use tracing::{error, info};

pub struct DiscordBridge {
    token: String,
    http: Arc<Http>,
}

impl DiscordBridge {
    pub fn new(token: String) -> Self {
        let http = Arc::new(Http::new(&token));
        Self { token, http }
    }
}

struct Handler {
    command_tx: tokio::sync::mpsc::Sender<ControlCommand>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, msg: DiscordMessage) {
        if msg.author.bot {
            return;
        }

        info!(
            "📩 [Discord] Received message from {}: {}",
            msg.author.name, msg.content
        );

        let cmd = ControlCommand::Chat {
            message: msg.content.clone(),
            channel_id: msg.channel_id.get(),
        };

        if let Err(e) = self.command_tx.send(cmd).await {
            error!("❌ [Discord] Failed to send command to Core relay: {:?}", e);
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("✅ [Discord] {} is connected!", ready.user.name);
    }
}

#[async_trait]
impl ChannelBridge for DiscordBridge {
    fn name(&self) -> &str {
        "Discord"
    }

    async fn send_message(&self, channel_id: &str, content: &str) -> Result<(), AiomeError> {
        let channel_id_u64: u64 = channel_id.parse().map_err(|_| AiomeError::Infrastructure {
            reason: "Invalid Discord Channel ID".to_string(),
        })?;
        let channel = serenity::model::id::ChannelId::new(channel_id_u64);

        // Help inference by explicitly specifying the error type or using a temporary variable
        let _ = channel
            .say(&self.http, content)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Discord send failed: {}", e),
            })?;

        Ok(())
    }

    async fn run(
        &self,
        command_tx: tokio::sync::mpsc::Sender<ControlCommand>,
    ) -> Result<(), AiomeError> {
        let intents = GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::DIRECT_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT;

        let handler = Handler { command_tx };

        let mut client = Client::builder(&self.token, intents)
            .event_handler(handler)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to create Discord client: {}", e),
            })?;

        info!("🚀 [Discord] Starting serenity gateway...");
        client
            .start()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Discord client error: {}", e),
            })?;

        Ok(())
    }
}
