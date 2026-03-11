/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use async_trait::async_trait;
use aiome_core::error::AiomeError;
use super::bridge_trait::ChannelBridge;
use tracing::{info, error};
use teloxide::prelude::*;
use shared::watchtower::ControlCommand;

pub struct TelegramBridge {
    token: String,
    bot: Bot,
}

impl TelegramBridge {
    pub fn new(token: String) -> Self {
        let bot = Bot::new(&token);
        Self { token, bot }
    }
}

#[async_trait]
impl ChannelBridge for TelegramBridge {
    fn name(&self) -> &str {
        "Telegram"
    }

    async fn send_message(&self, _channel_id: &str, content: &str) -> Result<(), AiomeError> {
        let chat_id: i64 = _channel_id.parse().map_err(|_| AiomeError::Infrastructure { reason: "Invalid Telegram Chat ID".to_string() })?;
        
        self.bot.send_message(ChatId(chat_id), content).await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Telegram send failed: {}", e) })?;
        
        Ok(())
    }

    async fn run(&self, command_tx: tokio::sync::mpsc::Sender<ControlCommand>) -> Result<(), AiomeError> {
        info!("🚀 [Telegram] Starting teloxide poller...");
        
        let tx = command_tx.clone();
        let handler = dptree::entry().branch(
            Update::filter_message().endpoint(move |_bot: Bot, msg: Message| {
                let tx = tx.clone();
                async move {
                    if let Some(text) = msg.text() {
                        info!("📩 [Telegram] Received message: {}", text);
                        let cmd = ControlCommand::Chat {
                            message: text.to_string(),
                            channel_id: msg.chat.id.0 as u64,
                        };
                        let _ = tx.send(cmd).await;
                    }
                    respond(())
                }
            })
        );

        Dispatcher::builder(self.bot.clone(), handler)
            .enable_ctrlc_handler()
            .build()
            .dispatch()
            .await;

        Ok(())
    }
}
