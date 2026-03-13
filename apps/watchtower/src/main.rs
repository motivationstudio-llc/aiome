/*
 * Aiome - Watchtower (The Soul)
 * Copyright (C) 2026 motivationstudio, LLC
 */

use tracing::{info, warn, error};
use infrastructure::channel_bridge::{DiscordBridge, TelegramBridge, ChannelBridge};
use shared::watchtower::{CoreEvent, ControlCommand};
use std::sync::Arc;
use tokio_tungstenite::connect_async;
use futures_util::{StreamExt, SinkExt};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("👁️ Starting Aiome Watchtower (Soul Bridge)...");

    let discord_token = std::env::var("DISCORD_TOKEN").ok();
    let telegram_token = std::env::var("TELEGRAM_TOKEN").ok();
    let api_secret = std::env::var("API_SERVER_SECRET").expect("🚨 API_SERVER_SECRET must be set for security!");
    let api_ws_url = std::env::var("API_WS_URL").unwrap_or_else(|_| "ws://127.0.0.1:3015/api/v1/watchtower/ws".to_string());

    let (command_tx, mut command_rx) = mpsc::channel::<ControlCommand>(100);

    let mut bridges: Vec<Arc<dyn ChannelBridge>> = Vec::new();

    if let Some(token) = discord_token {
        info!("🔌 Initializing Discord Bridge...");
        bridges.push(Arc::new(DiscordBridge::new(token)));
    }

    if let Some(token) = telegram_token {
        info!("🔌 Initializing Telegram Bridge...");
        bridges.push(Arc::new(TelegramBridge::new(token)));
    }

    if bridges.is_empty() {
        warn!("⚠️ No channel tokens found in environment. Watchtower will run in silent mode.");
    }

    let bridges = Arc::new(bridges);
    let bridges_clone = bridges.clone();

    // Spawn bridges
    for bridge in bridges_clone.iter() {
        let b = bridge.clone();
        let tx = command_tx.clone();
        tokio::spawn(async move {
            if let Err(e) = b.run(tx).await {
                error!("❌ Bridge {} error: {:?}", b.name(), e);
            }
        });
    }

    // Connect to Core (api-server)
    loop {
        info!("🔗 Connecting to Aiome Core at {}...", api_ws_url);
        
        let mut request = url::Url::parse(&api_ws_url)?.into_client_request()?;
        request.headers_mut().insert(
            "Authorization",
            format!("Bearer {}", api_secret).parse().map_err(|e| anyhow::anyhow!("Failed to parse auth header: {}", e))?
        );

        match connect_async(request).await {
            Ok((mut ws_stream, _)) => {
                info!("✅ Connected to Aiome Core.");
                
                loop {
                    tokio::select! {
                        msg = ws_stream.next() => {
                            match msg {
                                Some(Ok(Message::Text(text))) => {
                                    if let Ok(event) = serde_json::from_str::<CoreEvent>(&text) {
                                        handle_core_event(event, bridges.as_slice()).await;
                                    }
                                }
                                Some(Ok(Message::Close(_))) => break,
                                Some(Err(e)) => {
                                    error!("❌ WebSocket error: {:?}", e);
                                    break;
                                }
                                None => break,
                                _ => {}
                            }
                        }
                        cmd = command_rx.recv() => {
                            if let Some(command) = cmd {
                                if let Ok(json) = serde_json::to_string(&command) {
                                    if let Err(e) = ws_stream.send(Message::Text(json)).await {
                                        error!("❌ Failed to send command to Core: {:?}", e);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("❌ Failed to connect to Core: {:?}. Retrying in 10s...", e);
            }
        }
        
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}

async fn handle_core_event(event: CoreEvent, bridges: &[Arc<dyn ChannelBridge>]) {
    match event {
        CoreEvent::ChatResponse { response, channel_id, .. } => {
            info!("📨 Relaying ChatResponse to channel {}", channel_id);
            for bridge in bridges {
                let _ = bridge.send_message(&channel_id.to_string(), &response).await;
            }
        }
        CoreEvent::ProactiveTalk { message, channel_id } => {
            info!("📨 Relaying ProactiveTalk to channel {}", channel_id);
            let target_channel = if channel_id == 0 {
                std::env::var("DISCORD_CHAT_CHANNEL_ID").unwrap_or_else(|_| channel_id.to_string())
            } else {
                channel_id.to_string()
            };

            for bridge in bridges {
                let _ = bridge.send_message(&target_channel, &message).await;
            }
        }
        _ => {}
    }
}

use tokio_tungstenite::tungstenite::client::IntoClientRequest;
