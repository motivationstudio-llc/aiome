/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

pub mod bridge_trait;
pub mod discord;
pub mod telegram;

pub use bridge_trait::ChannelBridge;
pub use discord::DiscordBridge;
pub use telegram::TelegramBridge;
