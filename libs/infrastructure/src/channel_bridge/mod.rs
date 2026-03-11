/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

pub mod bridge_trait;
pub mod discord;
pub mod telegram;

pub use bridge_trait::ChannelBridge;
pub use discord::DiscordBridge;
pub use telegram::TelegramBridge;
