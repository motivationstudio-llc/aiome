use axum::{
    response::Json,
    extract::State,
    http::StatusCode,
};
use crate::AppState;
use aiome_core::biome::{BiomeMessage, BiomeDialogue, DialogueStatus, AutonomousBiomeEngine, AutonomousConfig};
use aiome_core::biome::dialogue::DialogueManager;
use aiome_core::traits::JobQueue;
use tracing::{info, warn, error};
use sqlx::Row;
use std::sync::Arc;
use std::sync::atomic::Ordering;

#[derive(serde::Deserialize)]
pub struct SendBiomeRequest {
    pub recipient_pubkey: String,
    pub topic_id: String,
    pub content: String,
}

#[derive(serde::Deserialize)]
pub struct StartAutonomousRequest {
    pub topic_id: String,
    pub peer_pubkey: String,
    pub interval_secs: Option<u64>,
    pub max_rounds: Option<u32>,
}

pub async fn biome_status(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let peer_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM biome_peers")
        .fetch_one(state.job_queue.get_pool()).await.unwrap_or(0);

    Json(serde_json::json!({
        "status": "online",
        "peer_count": peer_count,
        "message_ja": "Biome プロトコル準備完了。AI同士の対話を待機中...",
        "message_en": "Biome protocol ready. Waiting for AI-to-AI dialogue..."
    }))
}

pub async fn autonomous_start(
    State(state): State<AppState>,
    Json(req): Json<StartAutonomousRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if state.autonomous_running.load(Ordering::SeqCst) {
        return Err((StatusCode::CONFLICT, Json(serde_json::json!({"error": "Autonomous dialogue is already running"}))));
    }

    let config = AutonomousConfig {
        topic_id: req.topic_id,
        peer_pubkey: req.peer_pubkey,
        interval_secs: req.interval_secs.unwrap_or(30),
        max_rounds: req.max_rounds.unwrap_or(10),
    };

    state.autonomous_running.store(true, Ordering::SeqCst);
    let mut config_write = state.autonomous_config.write().await;
    *config_write = Some(config.clone());
    drop(config_write);

    let queue = state.job_queue.clone();
    let llm = state.provider.clone();
    let running = state.autonomous_running.clone();
    let semaphore = state.llm_semaphore.clone();

    tokio::spawn(async move {
        AutonomousBiomeEngine::start_loop(config, queue, llm, running, semaphore).await;
    });

    Ok(Json(serde_json::json!({"status": "started"})))
}

pub async fn autonomous_stop(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    state.autonomous_running.store(false, Ordering::SeqCst);
    Json(serde_json::json!({"status": "stopping"}))
}

pub async fn autonomous_status(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let running = state.autonomous_running.load(Ordering::SeqCst);
    let config = state.autonomous_config.read().await;
    
    Json(serde_json::json!({
        "running": running,
        "config": *config
    }))
}

pub async fn list_messages(
    State(state): State<AppState>,
) -> Json<Vec<serde_json::Value>> {
    let messages = state.job_queue.fetch_biome_messages("default", 100).await.unwrap_or_default();
    
    // If "default" is empty, try broadly or filter by topic if needed.
    // For MVP, if we want ALL messages:
    let rows = sqlx::query("SELECT * FROM biome_messages ORDER BY created_at DESC LIMIT 100")
        .fetch_all(state.job_queue.get_pool()).await.unwrap_or_default();
    
    let messages = rows.into_iter().map(|row| {
        serde_json::json!({
            "id": row.get::<i64, _>("id"),
            "sender_pubkey": row.get::<String, _>("sender_pubkey"),
            "recipient_pubkey": row.get::<String, _>("recipient_pubkey"),
            "topic_id": row.get::<String, _>("topic_id"),
            "content": row.get::<String, _>("content"),
            "karma_root_cid": row.get::<String, _>("karma_root_cid"),
            "signature": row.get::<String, _>("signature"),
            "lamport_clock": row.get::<i64, _>("lamport_clock"),
            "encryption": row.get::<String, _>("encryption"),
            "created_at": row.get::<Option<String>, _>("created_at"),
        })
    }).collect();

    Json(messages)
}

pub async fn send_message(
    State(state): State<AppState>,
    Json(req): Json<SendBiomeRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let sender_pubkey = state.job_queue.get_node_id().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;

    let clock = state.job_queue.tick_local_clock().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;

    // 0. Biome Dialogue Constraint Check
    if let Err(e) = DialogueManager::check_and_advance_turn(&*state.job_queue, &req.topic_id).await {
        warn!("🚫 [Biome] Message blocked: {}", e);
        return Err((StatusCode::FORBIDDEN, Json(serde_json::json!({"error": e.to_string()}))));
    }

    // Phase 20 MVP: Plaintext, Karma Root CID placeholder
    let karma_root = "cid_placeholder_v20".to_string();
    
    // 1. Sign the message
    let payload_to_sign = format!("{}:{}:{}", sender_pubkey, req.topic_id, clock);
    let signature = state.job_queue.sign_swarm_payload(&payload_to_sign).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;

    let msg = BiomeMessage {
        sender_pubkey,
        recipient_pubkey: req.recipient_pubkey,
        topic_id: req.topic_id,
        content: req.content,
        karma_root_cid: karma_root,
        signature: signature.clone(),
        lamport_clock: clock,
        timestamp: chrono::Utc::now().to_rfc3339(),
        encryption: "none".to_string(),
    };

    // 2. Relay via Hub
    let hub_url = std::env::var("SAMSARA_HUB_REST").unwrap_or_else(|_| "http://127.0.0.1:3016".to_string());
    let hub_secret = std::env::var("FEDERATION_SECRET").unwrap_or_else(|_| "dev_secret".to_string());
    let client = reqwest::Client::new();

    info!("🚀 [Biome] Sending message to Hub for relay (Topic: {})", msg.topic_id);
    let res = client.post(format!("{}/api/v1/biome/relay", hub_url))
        .header("Authorization", format!("Bearer {}", hub_secret))
        .json(&msg)
        .send().await;

    match res {
        Ok(r) if r.status().is_success() => {
            // Store a copy in local history
            let _ = sqlx::query("INSERT INTO biome_messages (sender_pubkey, recipient_pubkey, topic_id, content, karma_root_cid, signature, lamport_clock, encryption) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(&msg.sender_pubkey).bind(&msg.recipient_pubkey).bind(&msg.topic_id).bind(&msg.content).bind(&msg.karma_root_cid).bind(&signature).bind(msg.lamport_clock as i64).bind(&msg.encryption)
                .execute(state.job_queue.get_pool()).await;

            Ok(Json(serde_json::json!({"status": "sent", "topic_id": msg.topic_id})))
        },
        _ => {
            // Hub unavailable or failed: Fallback to local
            warn!("⚠️ [Biome] Hub relay failed. Saving message locally as fallback.");
            let _ = sqlx::query("INSERT INTO biome_messages (sender_pubkey, recipient_pubkey, topic_id, content, karma_root_cid, signature, lamport_clock, encryption) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(&msg.sender_pubkey).bind(&msg.recipient_pubkey).bind(&msg.topic_id).bind(&msg.content).bind(&msg.karma_root_cid).bind(&signature).bind(msg.lamport_clock as i64).bind(&msg.encryption)
                .execute(state.job_queue.get_pool()).await;
            
            Ok(Json(serde_json::json!({"status": "sent_local_only", "topic_id": msg.topic_id})))
        }
    }
}
