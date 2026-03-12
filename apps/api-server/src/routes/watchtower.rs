/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
};
use tracing::{info, warn, error};
use futures_util::{sink::SinkExt, stream::StreamExt};
use crate::AppState;
use shared::watchtower::{CoreEvent, ControlCommand};
use aiome_core::traits::JobQueue;
use crate::routes::agent::{AgentChatRequest};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut broadcast_rx = state.event_sender.subscribe();

    info!("👁️ [WatchtowerWS] Client connected.");

    // Task 1: Relay CoreEvents from Broadcast to WS
    let mut relay_task = tokio::spawn(async move {
        while let Ok(event) = broadcast_rx.recv().await {
            if let Ok(json) = serde_json::to_string(&event) {
                if sender.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Task 2: Relay ControlCommands from WS to Core
    let mut command_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            if let Ok(command) = serde_json::from_str::<ControlCommand>(&text) {
                info!("🎮 [WatchtowerWS] Received command: {:?}", command);
                match command {
                    ControlCommand::Chat { message, channel_id } => {
                        let state_clone = state.clone();
                        tokio::spawn(async move {
                            let payload = AgentChatRequest {
                                prompt: message,
                                history: vec![],
                                channel_id: Some(channel_id.to_string()),
                            };
                            
                            if let Err(e) = handle_chat_command(state_clone, payload).await {
                                error!("❌ [WatchtowerWS] Chat processing failed: {:?}", e);
                            }
                        });
                    },
                    ControlCommand::GetAgentStats => {
                        if let Ok(stats) = state.job_queue.get_agent_stats().await {
                            let _ = state.event_sender.send(CoreEvent::AgentStatsResponse(stats));
                        }
                    },
                    _ => {
                        warn!("⚠️ [WatchtowerWS] Command not implemented yet: {:?}", command);
                    }
                }
            }
        }
    });

    tokio::select! {
        _ = (&mut relay_task) => {
            command_task.abort();
        },
        _ = (&mut command_task) => {
            relay_task.abort();
        },
    }
    info!("👁️ [WatchtowerWS] Client disconnected.");
}

async fn handle_chat_command(state: AppState, payload: AgentChatRequest) -> anyhow::Result<()> {
    use crate::routes::agent::{read_workspace_file, build_system_instructions};
    use aiome_core::traits::JobQueue;
    use tokio::time::timeout;
    use std::time::Duration;

    let channel_id = payload.channel_id.unwrap_or_else(|| "0".to_string());
    let channel_id_u64: u64 = channel_id.parse().unwrap_or(0);

    // 1. Guardrails
    if let shared::guardrails::ValidationResult::Blocked(reason) = shared::guardrails::validate_input(&payload.prompt) {
        let _ = state.event_sender.send(CoreEvent::ChatResponse {
            response: format!("🚨 [GUARDRAIL BLOCK] {}", reason),
            channel_id: channel_id_u64,
            resource_path: None,
        });
        return Ok(());
    }

    // 2. Persist
    let _ = state.job_queue.insert_chat_message(&channel_id, "user", &payload.prompt).await;

    // 3. Build Prompt (minimal version for now)
    let summary = None; // simplified
    let karma_str = "Watchtower context active.";

    let ai_name = state.job_queue.get_setting_value("ai_name").await.ok().flatten();
    let system_instructions = build_system_instructions(&state, karma_str, summary, ai_name);
    let full_prompt = format!(
        "{}\nUSER: {}\nAI: ", 
        system_instructions, 
        payload.prompt
    );

    // 4. LLM Call
    let _llm_permit = state.llm_semaphore.acquire().await.ok();
    match timeout(Duration::from_secs(120), state.provider.complete(&full_prompt, None)).await {
        Ok(Ok(reply)) => {
            let reply = reply.trim().to_string();
            let _ = state.job_queue.insert_chat_message(&channel_id, "assistant", &reply).await;
            let _ = state.event_sender.send(CoreEvent::ChatResponse {
                response: reply,
                channel_id: channel_id_u64,
                resource_path: None,
            });
        }
        _ => {
            let _ = state.event_sender.send(CoreEvent::ChatResponse {
                response: "Error: Cognitive engine timeout or failure.".to_string(),
                channel_id: channel_id_u64,
                resource_path: None,
            });
        }
    }

    Ok(())
}
