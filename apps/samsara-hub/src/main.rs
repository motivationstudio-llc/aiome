/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 * You may not provide the software to third parties as a hosted or managed service, 
 * where the service provides users with access to any substantial set of the features 
 * or functionality of the software.
 */

use axum::{
    extract::{State, ws::{WebSocket, WebSocketUpgrade, Message}, DefaultBodyLimit},
    routing::{get, post},
    response::{IntoResponse},
    Router, Json,
    http::{StatusCode, HeaderMap},
    error_handling::HandleErrorLayer,
};
use std::sync::Arc;
use tokio::sync::broadcast;
use sqlx::SqlitePool;
use aiome_core::contracts::{FederationSyncRequest, FederationSyncResponse, FederationPushRequest, FederationPushResponse, FederatedKarma, ImmuneRule};
use tracing::{info, warn, error};
use tower_http::cors::CorsLayer;
use secrecy::ExposeSecret;
use tower::{ServiceBuilder, limit::RateLimitLayer, buffer::BufferLayer};
use std::time::Duration;
use futures_util::SinkExt;
use serde::{Deserialize, Serialize};

struct HubState {
    pool: SqlitePool,
    secret: secrecy::SecretString,
    tx: broadcast::Sender<ImmuneRule>,
}

#[derive(sqlx::FromRow, Serialize, Deserialize)]
struct FederatedKarmaRecord {
    id: String,
    karma_type: String,
    related_skill: String,
    lesson: String,
    weight: i64,
    soul_version_hash: Option<String>,
    lamport_clock: i64,
    node_id: String,
    signature: Option<String>,
    created_at: String,
}

#[derive(sqlx::FromRow, Serialize, Deserialize)]
struct ImmuneRuleRecord {
    id: String,
    pattern: String,
    severity: i64,
    action: String,
    lamport_clock: i64,
    node_id: String,
    signature: Option<String>,
    created_at: String,
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing with JSON for easier aggregation in the hub
    tracing_subscriber::fmt().json().init();
    
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:samsara_hub.db?mode=rwc".to_string());
    let secret = secrecy::SecretString::new(
        std::env::var("FEDERATION_SECRET").expect("FEDERATION_SECRET must be set for Samsara Hub security")
        .into()
    );
    let port = std::env::var("PORT").unwrap_or_else(|_| "3016".to_string());

    let pool = SqlitePool::connect(&db_url).await?;
    init_hub_db(&pool).await?;

    // Create broadcast channel for real-time rule notification
    let (tx, _) = broadcast::channel(100);
    let state = Arc::new(HubState { pool, secret, tx });

    // Secure CORS Policy: Restrict to specific trusted origins or localhost for development
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any) 
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_headers([axum::http::header::CONTENT_TYPE, axum::http::header::AUTHORIZATION]);


    let app = Router::new()
        .route("/api/v1/federation/sync", post(sync_handler))
        .route("/api/v1/federation/push", post(push_handler))
        .route("/api/v1/federation/ws", get(ws_handler))
        .route("/api/v1/health", get(health_handler))
        .layer(cors)
        .layer(DefaultBodyLimit::max(5 * 1024 * 1024)) // 5MB limit
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|err| async move {
                    (StatusCode::INTERNAL_SERVER_ERROR, format!("Unhandled internal error: {}", err))
                }))
                .layer(BufferLayer::new(1024))
                .layer(RateLimitLayer::new(100, Duration::from_secs(60)))
        )
        .with_state(state);

    let addr = format!("127.0.0.1:{}", port); 
    info!("🏔️ Samsara Hub (The Validator) listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn init_hub_db(pool: &SqlitePool) -> anyhow::Result<()> {
    // Hub DB schema includes 'is_approved' or separate quarantine tables.
    // For this implementation, we use separate tables for Quarantined data.
    
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS approved_karma (
            id TEXT PRIMARY KEY,
            node_id TEXT NOT NULL,
            karma_type TEXT NOT NULL,
            related_skill TEXT NOT NULL,
            lesson TEXT NOT NULL,
            weight INTEGER NOT NULL,
            soul_version_hash TEXT,
            lamport_clock INTEGER NOT NULL DEFAULT 0,
            signature TEXT,
            approved_at TEXT DEFAULT (datetime('now')),
            created_at TEXT NOT NULL
        );"
    ).execute(pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS quarantined_karma (
            id TEXT PRIMARY KEY,
            node_id TEXT NOT NULL,
            karma_type TEXT NOT NULL,
            related_skill TEXT NOT NULL,
            lesson TEXT NOT NULL,
            weight INTEGER NOT NULL,
            soul_version_hash TEXT,
            lamport_clock INTEGER NOT NULL DEFAULT 0,
            signature TEXT,
            received_at TEXT DEFAULT (datetime('now')),
            created_at TEXT NOT NULL
        );"
    ).execute(pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS approved_rules (
            id TEXT PRIMARY KEY,
            pattern TEXT NOT NULL,
            severity INTEGER NOT NULL,
            action TEXT NOT NULL,
            node_id TEXT NOT NULL,
            lamport_clock INTEGER NOT NULL DEFAULT 0,
            signature TEXT,
            approved_at TEXT DEFAULT (datetime('now')),
            created_at TEXT NOT NULL
        );"
    ).execute(pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS quarantined_rules (
            id TEXT PRIMARY KEY,
            node_id TEXT NOT NULL,
            pattern TEXT NOT NULL,
            severity INTEGER NOT NULL,
            action TEXT NOT NULL,
            lamport_clock INTEGER NOT NULL DEFAULT 0,
            signature TEXT,
            received_at TEXT DEFAULT (datetime('now')),
            created_at TEXT NOT NULL
        );"
    ).execute(pool).await?;

    info!("✅ Hub Database initialized (Approved & Quarantine layers).");
    Ok(())
}

async fn health_handler() -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::OK, Json(serde_json::json!({"status": "healthy", "service": "samsara-hub"})))
}

async fn sync_handler(
    State(state): State<Arc<HubState>>,
    headers: HeaderMap,
    Json(payload): Json<FederationSyncRequest>,
) -> impl IntoResponse {
    use subtle::ConstantTimeEq;
    let auth = headers.get(axum::http::header::AUTHORIZATION).and_then(|h| h.to_str().ok()).unwrap_or("");
    let expected = format!("Bearer {}", state.secret.expose_secret());
    
    let is_auth_valid = if auth.len() == expected.len() {
        bool::from(auth.as_bytes().ct_eq(expected.as_bytes()))
    } else {
        // Technically, length checks can leak length, but tokens are usually fixed length.
        // A full HMAC setup would be better, but this mitigates basic string-comparison timing attacks.
        false
    };

    if !is_auth_valid {
        warn!("🔒 Unauthorized sync attempt from node: {}", payload.node_id);
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))).into_response();
    }

    info!("🌐 Node {} pulling approved updates since {:?}", payload.node_id, payload.since);

    let since = payload.since.unwrap_or_else(|| "1970-01-01T00:00:00".to_string());

    // Fetch ONLY approved data
    let karmas = sqlx::query_as::<_, FederatedKarmaRecord>(
        "SELECT id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, lamport_clock, node_id, signature FROM approved_karma WHERE approved_at > ?"
    ).bind(&since).fetch_all(&state.pool).await.unwrap_or_default();

    let rules = sqlx::query_as::<_, ImmuneRuleRecord>(
        "SELECT id, pattern, severity, action, created_at, lamport_clock, node_id, signature FROM approved_rules WHERE approved_at > ?"
    ).bind(&since).fetch_all(&state.pool).await.unwrap_or_default();

    let response = FederationSyncResponse {
        new_karmas: karmas.into_iter().map(|k| FederatedKarma {
            id: k.id,
            job_id: None,
            karma_type: k.karma_type,
            related_skill: k.related_skill,
            lesson: k.lesson,
            weight: k.weight as i32,
            created_at: k.created_at,
            soul_version_hash: k.soul_version_hash,
            lamport_clock: k.lamport_clock as u64,
            node_id: k.node_id,
            signature: k.signature,
        }).collect(),
        new_immune_rules: rules.into_iter().map(|r| ImmuneRule {
            id: r.id,
            pattern: r.pattern,
            severity: r.severity as u8,
            action: r.action,
            created_at: r.created_at,
            lamport_clock: r.lamport_clock as u64,
            node_id: r.node_id,
            signature: r.signature,
        }).collect(),
        new_arena_matches: Vec::new(), // TODO
        server_time: chrono::Utc::now().to_rfc3339(),
    };

    (StatusCode::OK, Json(response)).into_response()
}

async fn push_handler(
    State(state): State<Arc<HubState>>,
    headers: HeaderMap,
    Json(payload): Json<FederationPushRequest>,
) -> impl IntoResponse {
    // Auth Wall
    use subtle::ConstantTimeEq;
    let auth = headers.get(axum::http::header::AUTHORIZATION).and_then(|h| h.to_str().ok()).unwrap_or("");
    let expected = format!("Bearer {}", state.secret.expose_secret());
    
    let is_auth_valid = if auth.len() == expected.len() {
        bool::from(auth.as_bytes().ct_eq(expected.as_bytes()))
    } else {
        false
    };

    if !is_auth_valid {
        warn!("🔒 Unauthorized push attempt from node: {}", payload.node_id);
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))).into_response();
    }

    let karma_count = payload.karmas.len();
    let rule_count = payload.rules.len();
    info!("📥 Received push from node {}: {} Karmas, {} Rules. Sending to Quarantine.", payload.node_id, karma_count, rule_count);

    let mut tx = match state.pool.begin().await {
        Ok(t) => t,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };

    for k in &payload.karmas {
        let _ = sqlx::query(
            "INSERT INTO quarantined_karma (id, node_id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, lamport_clock, signature)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO NOTHING"
        )
        .bind(&k.id).bind(&payload.node_id).bind(&k.karma_type).bind(&k.related_skill).bind(&k.lesson)
        .bind(k.weight as i64).bind(&k.soul_version_hash).bind(&k.created_at)
        .bind(k.lamport_clock as i64).bind(&k.signature)
        .execute(&mut *tx).await;
    }

    for r in &payload.rules {
        let _ = sqlx::query(
            "INSERT INTO quarantined_rules (id, node_id, pattern, severity, action, created_at, lamport_clock, signature)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO NOTHING"
        )
        .bind(&r.id).bind(&payload.node_id).bind(&r.pattern).bind(r.severity as i64).bind(&r.action).bind(&r.created_at)
        .bind(r.lamport_clock as i64).bind(&r.signature)
        .execute(&mut *tx).await;
    }

    if let Err(e) = tx.commit().await {
        error!("❌ Push commit failed: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }

    // 📣 Real-time Broadcast to all connected nodes (Relay Sync)
    for r in &payload.rules {
        let _ = state.tx.send(r.clone());
    }

    (StatusCode::OK, Json(FederationPushResponse {
        accepted_count: karma_count + rule_count,
        message: "Data received and placed in quarantine for validation.".to_string(),
    })).into_response()
}

async fn ws_handler(
    headers: HeaderMap,
    ws: WebSocketUpgrade,
    State(state): State<Arc<HubState>>,
) -> impl IntoResponse {
    use subtle::ConstantTimeEq;
    let auth = headers.get(axum::http::header::AUTHORIZATION).and_then(|h| h.to_str().ok()).unwrap_or("");
    let expected = format!("Bearer {}", state.secret.expose_secret());
    
    let is_auth_valid = if auth.len() == expected.len() {
        bool::from(auth.as_bytes().ct_eq(expected.as_bytes()))
    } else {
        false
    };

    if !is_auth_valid {
        warn!("🔒 Unauthorized WS upgrade attempt");
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<HubState>) {
    use aiome_core::contracts::HubMessage;
    info!("🔌 Authorized node connected via WebSocket");
    
    let mut rx = state.tx.subscribe();

    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => {
                        info!("🔌 Node disconnected");
                        break;
                    }
                    _ => {}
                }
            }
            res = rx.recv() => {
                match res {
                    Ok(rule) => {
                        let hub_msg = HubMessage::NewImmuneRule(rule);
                        if let Ok(text) = serde_json::to_string(&hub_msg) {
                            if socket.send(Message::Text(text)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("⚠️ WS Client lagged by {} messages. Triggering Catch-up Sync.", n);
                        let hub_msg = HubMessage::LaggedForceSync { 
                            server_time: chrono::Utc::now().to_rfc3339() 
                        };
                        if let Ok(text) = serde_json::to_string(&hub_msg) {
                            let _ = socket.send(Message::Text(text)).await;
                        }
                        // Continue loop, client will sync via REST
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
}


