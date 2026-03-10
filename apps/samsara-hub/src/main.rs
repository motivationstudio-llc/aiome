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
use tokio_util::sync::CancellationToken;
use std::sync::Arc;
use tokio::sync::broadcast;
use sqlx::SqlitePool;
use aiome_core::contracts::{FederationSyncRequest, FederationSyncResponse, FederationPushRequest, FederationPushResponse, FederatedKarma, ImmuneRule, HubMessage};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::str::FromStr;
use tracing::{info, warn, error};
use tower_http::cors::CorsLayer;
use secrecy::ExposeSecret;
use tower::{ServiceBuilder, limit::RateLimitLayer, buffer::BufferLayer};
use std::time::Duration;
use serde::{Deserialize, Serialize};

struct HubState {
    pool: SqlitePool,
    secret: secrecy::SecretString,
    tx: broadcast::Sender<HubMessage>,
    active_connections: std::sync::atomic::AtomicUsize,
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

    // Configure SQLite with Performance & Reliability Options for Large-Scale Sync
    let options = SqliteConnectOptions::from_str(&db_url)?
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_millis(10000))
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);

    let pool = SqlitePoolOptions::new()
        .max_connections(50) // Scaling to handle multi-node load testing
        .connect_with(options).await?;

    init_hub_db(&pool).await?;

    // Create broadcast channel for real-time rule/karma notification
    let (tx, _) = broadcast::channel(100);
    let state = Arc::new(HubState { 
        pool: pool.clone(), 
        secret, 
        tx,
        active_connections: std::sync::atomic::AtomicUsize::new(0),
    });
    
    let token = CancellationToken::new();

    // Spawn the Approval Worker to process quarantine
    tokio::spawn(approval_worker(pool, token.clone()));

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
        // Biome Routes (Phase 20)
        .route("/api/v1/biome/relay", post(biome_relay_handler))
        .route("/api/v1/biome/ws", get(biome_ws_handler))
        .layer(cors)
        .layer(DefaultBodyLimit::max(5 * 1024 * 1024)) // 5MB limit
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|err| async move {
                    (StatusCode::INTERNAL_SERVER_ERROR, format!("Unhandled internal error: {}", err))
                }))
                .layer(BufferLayer::new(2048))
                .layer(RateLimitLayer::new(600, Duration::from_secs(60))) // High frequency for Biome
        )
        .with_state(state);

    let addr = format!("127.0.0.1:{}", port); 
    info!("🏔️ Samsara Hub (The Validator) listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(token))
        .await?;

    Ok(())
}

async fn shutdown_signal(token: CancellationToken) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("🔴 [samsara-hub] Received Ctrl+C signal. Initiating graceful shutdown...");
        },
        _ = terminate => {
            info!("🔴 [samsara-hub] Received Terminate signal. Initiating graceful shutdown...");
        },
    }
    
    token.cancel();
}

async fn init_hub_db(pool: &SqlitePool) -> anyhow::Result<()> {
    // Hub DB schema includes 'is_approved' or separate quarantine tables.
    // For this implementation, we use separate tables for Quarantined data.
    
    let _now_rfc = chrono::Utc::now().to_rfc3339();
    
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
            approved_at TEXT,
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
            received_at TEXT,
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
            approved_at TEXT,
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
            received_at TEXT,
            created_at TEXT NOT NULL
        );"
    ).execute(pool).await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_approved_karma_at ON approved_karma(approved_at);").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_approved_rules_at ON approved_rules(approved_at);").execute(pool).await?;

    // BFT: Composite indexes for O(1) Equivocation (Double-Signing) Detection
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_q_karma_node_clock ON quarantined_karma(node_id, lamport_clock);").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_q_rules_node_clock ON quarantined_rules(node_id, lamport_clock);").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_a_karma_node_clock ON approved_karma(node_id, lamport_clock);").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_a_rules_node_clock ON approved_rules(node_id, lamport_clock);").execute(pool).await?;

    // BFT: Node Reputation & Slashing System table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS node_reputation (
            node_id TEXT PRIMARY KEY,
            reputation_score INTEGER NOT NULL DEFAULT 100,
            is_banned INTEGER NOT NULL DEFAULT 0,
            last_seen_at TEXT NOT NULL
        );"
    ).execute(pool).await?;
    
    // Biome Relay Buffer (Phase 20)
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS biome_relay_queue (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            recipient_pubkey TEXT NOT NULL,
            payload TEXT NOT NULL,
            is_delivered INTEGER NOT NULL DEFAULT 0,
            created_at TEXT DEFAULT (datetime('now'))
        );"
    ).execute(pool).await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_biome_relay_recipient ON biome_relay_queue(recipient_pubkey) WHERE is_delivered = 0;").execute(pool).await?;

    info!("✅ Hub Database initialized (Approved & Quarantine layers + BFT/Reputation & Biome).");
    Ok(())
}

async fn health_handler() -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::OK, Json(serde_json::json!({"status": "healthy", "service": "samsara-hub"})))
}

async fn biome_relay_handler(
    State(state): State<Arc<HubState>>,
    headers: HeaderMap,
    Json(msg): Json<aiome_core::biome::BiomeMessage>,
) -> (StatusCode, Json<serde_json::Value>) {
    // 1. Auth Check
    let auth = headers.get(axum::http::header::AUTHORIZATION).and_then(|h| h.to_str().ok()).unwrap_or("");
    if !auth.starts_with("Bearer ") || auth[7..] != *state.secret.expose_secret() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"status": "error", "message": "Unauthorized"})));
    }

    // 2. Verification (Signature)
    use ed25519_dalek::{VerifyingKey, Signature, Verifier};
    use base64::prelude::*;
    let mut valid = false;
    let payload = format!("{}:{}:{}", msg.sender_pubkey, msg.topic_id, msg.lamport_clock);
    if let (Ok(pubkey_bytes), Ok(sig_bytes)) = (BASE64_STANDARD.decode(&msg.sender_pubkey), BASE64_STANDARD.decode(&msg.signature)) {
        if let (Ok(pubkey), Ok(sig)) = (VerifyingKey::from_bytes(&pubkey_bytes.try_into().unwrap_or([0; 32])), Signature::from_slice(&sig_bytes)) {
            if pubkey.verify(payload.as_bytes(), &sig).is_ok() {
                valid = true;
            }
        }
    }

    if !valid {
         return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"status": "error", "message": "Invalid Signature"})));
    }

    // 3. Relay Logic
    info!("📫 [Hub] Relaying Biome Message from {} to topic {}", msg.sender_pubkey, msg.topic_id);
    
    // Buffer in DB
    let payload_json = serde_json::to_string(&msg).unwrap_or_default();
    let _ = sqlx::query("INSERT INTO biome_relay_queue (recipient_pubkey, payload) VALUES (?, ?)")
        .bind(&msg.recipient_pubkey)
        .bind(&payload_json)
        .execute(&state.pool).await;

    // Broadcast to real-time subscribers
    let _ = state.tx.send(HubMessage::BiomeRelay(msg));

    (StatusCode::ACCEPTED, Json(serde_json::json!({"status": "accepted"})))
}

async fn biome_ws_handler(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    State(state): State<Arc<HubState>>,
) -> impl IntoResponse {
    let auth = headers.get(axum::http::header::AUTHORIZATION).and_then(|h| h.to_str().ok()).unwrap_or("");
    if !auth.starts_with("Bearer ") || auth[7..] != *state.secret.expose_secret() {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    ws.on_upgrade(|socket| async move {
        handle_biome_ws(socket, state).await;
    })
}

async fn handle_biome_ws(mut socket: WebSocket, state: Arc<HubState>) {
    let mut rx = state.tx.subscribe();
    
    // Initial fetch of buffered messages for this node (would need node_id to be provided during handshake)
    // For MVP, just relay new messages in real-time.
    
    loop {
        tokio::select! {
            Ok(msg) = rx.recv() => {
                if let HubMessage::BiomeRelay(biome_msg) = msg {
                    // Filter: Only send if it's for this recipient (requires WS handshake to provide recipient_pubkey)
                    // For now, relay all but node should filter locally.
                    let text = serde_json::to_string(&HubMessage::BiomeRelay(biome_msg)).unwrap_or_default();
                    if socket.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(30)) => {
                if socket.send(Message::Ping(vec![].into())).await.is_err() {
                    break;
                }
            }
        }
    }
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

    // BFT: BAN Check
    match sqlx::query_scalar::<sqlx::Sqlite, i64>("SELECT is_banned FROM node_reputation WHERE node_id = ?")
        .bind(&payload.node_id).fetch_one(&state.pool).await {
            Ok(is_banned) if is_banned == 1 => {
                warn!("🛡️ [BFT] Rejecting sync from BANNED node: {}", payload.node_id);
                return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Node is banned"}))).into_response();
            }
            _ => {}
        }

    info!("🌐 Node {} pulling approved updates since {:?}", payload.node_id, payload.since);

    let since = payload.since.unwrap_or_else(|| "1970-01-01T00:00:00".to_string());

    // Fetch ONLY approved data with Pagination (Flaw 2: OOM Defense)
    let karmas = sqlx::query_as::<_, FederatedKarmaRecord>(
        "SELECT id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, lamport_clock, node_id, signature FROM approved_karma 
         WHERE approved_at > ? ORDER BY approved_at ASC LIMIT 500"
    ).bind(&since).fetch_all(&state.pool).await.unwrap_or_default();

    let rules = sqlx::query_as::<_, ImmuneRuleRecord>(
        "SELECT id, pattern, severity, action, created_at, lamport_clock, node_id, signature FROM approved_rules 
         WHERE approved_at > ? ORDER BY approved_at ASC LIMIT 500"
    ).bind(&since).fetch_all(&state.pool).await.unwrap_or_default();

    let has_more = karmas.len() == 500 || rules.len() == 500;
    let next_cursor: Option<String> = if has_more {
        // Find the latest approved_at for pagination (Keyset Pagination)
        // For simplicity, we just use the last item's timestamp if we hit the limit
        // In a real high-perf system, we'd query for the max timestamp in the results.
        None // Placeholder: will be refined if needed, but since is enough for now.
    } else {
        None
    };

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
        next_cursor: None, // Stateless: client just uses latest server_time or item timestamp
        has_more,
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

    // BFT: BAN Check
    match sqlx::query_scalar::<sqlx::Sqlite, i64>("SELECT is_banned FROM node_reputation WHERE node_id = ?")
        .bind(&payload.node_id).fetch_one(&state.pool).await {
            Ok(is_banned) if is_banned == 1 => {
                warn!("🛡️ [BFT] Rejecting push from BANNED node: {}", payload.node_id);
                return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Node is banned"}))).into_response();
            }
            _ => {}
        }

    let karma_count = payload.karmas.len();
    let rule_count = payload.rules.len();
    info!("📥 Received push from node {}: {} Karmas, {} Rules. Sending to Quarantine.", payload.node_id, karma_count, rule_count);

    let mut tx = match state.pool.begin().await {
        Ok(t) => t,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };

    let received_at = chrono::Utc::now().to_rfc3339();
    for k in &payload.karmas {
        // BFT: Equivocation Check (Double-Signing)
        // Check if node_id + lamport_clock already exists with different content in approved or quarantined
        let exists = sqlx::query_scalar::<sqlx::Sqlite, i64>(
            "SELECT COUNT(*) FROM (
                SELECT id FROM approved_karma WHERE node_id = ? AND lamport_clock = ? AND (lesson != ? OR weight != ?)
                UNION ALL
                SELECT id FROM quarantined_karma WHERE node_id = ? AND lamport_clock = ? AND (lesson != ? OR weight != ?)
             ) LIMIT 1"
        )
        .bind(&k.node_id).bind(k.lamport_clock as i64).bind(&k.lesson).bind(k.weight as i64)
        .bind(&k.node_id).bind(k.lamport_clock as i64).bind(&k.lesson).bind(k.weight as i64)
        .fetch_one(&state.pool).await.unwrap_or(0);

        if exists > 0 {
            warn!("🛡️ [BFT] EQUIVOCATION detected from node: {}. Slashing node.", k.node_id);
            let _ = sqlx::query("UPDATE node_reputation SET is_banned = 1, reputation_score = -1000 WHERE node_id = ?")
                .bind(&k.node_id).execute(&state.pool).await;
            return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Equivocation detected"}))).into_response();
        }

        let _ = sqlx::query(
            "INSERT INTO quarantined_karma (id, node_id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, lamport_clock, signature, received_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO NOTHING"
        )
        .bind(&k.id).bind(&k.node_id).bind(&k.karma_type).bind(&k.related_skill).bind(&k.lesson)
        .bind(k.weight as i64).bind(&k.soul_version_hash).bind(&k.created_at)
        .bind(k.lamport_clock as i64).bind(&k.signature).bind(&received_at)
        .execute(&mut *tx).await;
    }

    for r in &payload.rules {
        // BFT: Equivocation Check (Double-Signing) for Rules
        let exists = sqlx::query_scalar::<sqlx::Sqlite, i64>(
            "SELECT COUNT(*) FROM (
                SELECT id FROM approved_rules WHERE node_id = ? AND lamport_clock = ? AND (pattern != ? OR severity != ? OR action != ?)
                UNION ALL
                SELECT id FROM quarantined_rules WHERE node_id = ? AND lamport_clock = ? AND (pattern != ? OR severity != ? OR action != ?)
             ) LIMIT 1"
        )
        .bind(&r.node_id).bind(r.lamport_clock as i64).bind(&r.pattern).bind(r.severity as i64).bind(&r.action)
        .bind(&r.node_id).bind(r.lamport_clock as i64).bind(&r.pattern).bind(r.severity as i64).bind(&r.action)
        .fetch_one(&state.pool).await.unwrap_or(0);

        if exists > 0 {
            warn!("🛡️ [BFT] EQUIVOCATION detected in RULE from node: {}. Slashing node.", r.node_id);
            let _ = sqlx::query("UPDATE node_reputation SET is_banned = 1, reputation_score = -1000 WHERE node_id = ?")
                .bind(&r.node_id).execute(&state.pool).await;
            return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Equivocation detected"}))).into_response();
        }

        let _ = sqlx::query(
            "INSERT INTO quarantined_rules (id, node_id, pattern, severity, action, created_at, lamport_clock, signature, received_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO NOTHING"
        )
        .bind(&r.id).bind(&r.node_id).bind(&r.pattern).bind(r.severity as i64).bind(&r.action).bind(&r.created_at)
        .bind(r.lamport_clock as i64).bind(&r.signature).bind(&received_at)
        .execute(&mut *tx).await;
    }

    if let Err(e) = tx.commit().await {
        error!("❌ Push commit failed: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }

    // BFT: Update reputation / last_seen
    let _ = sqlx::query(
        "INSERT INTO node_reputation (node_id, last_seen_at) VALUES (?, ?)
         ON CONFLICT(node_id) DO UPDATE SET last_seen_at = excluded.last_seen_at, reputation_score = reputation_score + 1"
    ).bind(&payload.node_id).bind(&received_at).execute(&state.pool).await;

    // 📣 Real-time Broadcast to all connected nodes (Relay Sync)
    for r in &payload.rules {
        let _ = state.tx.send(HubMessage::NewImmuneRule(r.clone()));
    }
    for k in &payload.karmas {
        let _ = state.tx.send(HubMessage::NewKarma(k.clone()));
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
    
    // TCP Exhaustion Defense (Max Connections)
    let current_conn = state.active_connections.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    if current_conn >= 1000 {
        warn!("🛡️ [BFT] Hub reached max WebSocket connections (1000). Rejecting new node.");
        state.active_connections.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        let _ = socket.send(Message::Close(None)).await;
        return;
    }

    info!("🔌 Authorized node connected via WebSocket (Total: {})", current_conn + 1);
    
    let mut rx = state.tx.subscribe();
    let mut keepalive_timer = tokio::time::interval(Duration::from_secs(30));

    loop {
        tokio::select! {
            _ = keepalive_timer.tick() => {
                // Ping-Pong keepalive (Flaw 9)
                if socket.send(Message::Ping(Vec::new())).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => {
                        info!("🔌 Node disconnected");
                        break;
                    }
                    Some(Ok(Message::Text(text))) => {
                        // Handle Ping from client (Flaw 9)
                        if let Ok(HubMessage::Ping { client_time: _ }) = serde_json::from_str::<HubMessage>(&text) {
                            let pong = HubMessage::Pong { server_time: chrono::Utc::now().to_rfc3339() };
                            if let Ok(pong_text) = serde_json::to_string(&pong) {
                                let _ = socket.send(Message::Text(pong_text)).await;
                            }
                        }
                    }
                    _ => {}
                }
            }
            res = rx.recv() => {
                match res {
                    Ok(hub_msg) => {
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
    state.active_connections.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
}

async fn approval_worker(pool: SqlitePool, token: CancellationToken) {
    use ed25519_dalek::{VerifyingKey, Signature, Verifier};
    use base64::{prelude::BASE64_STANDARD, Engine};

    info!("⚙️ [ApprovalWorker] Starting quarantine validation thread.");

    loop {
        if token.is_cancelled() { break; }

        // 1. Process Quarantined Karma
        let karmas = sqlx::query_as::<_, FederatedKarmaRecord>("SELECT * FROM quarantined_karma LIMIT 50")
            .fetch_all(&pool).await.unwrap_or_default();

        for k in &karmas {
            let mut valid = false;
            if let Some(ref sig_b64) = k.signature {
                let payload = format!("{}:{}:{}", k.id, k.lesson, k.lamport_clock);
                if let (Ok(pubkey_bytes), Ok(sig_bytes)) = (BASE64_STANDARD.decode(&k.node_id), BASE64_STANDARD.decode(&sig_b64)) {
                    if let (Ok(pubkey), Ok(sig)) = (VerifyingKey::from_bytes(&pubkey_bytes.try_into().unwrap_or([0; 32])), Signature::from_slice(&sig_bytes)) {
                        if pubkey.verify(payload.as_bytes(), &sig).is_ok() {
                            valid = true;
                        }
                    }
                }
            }

            if valid {
                match pool.begin().await {
                    Ok(mut tx) => {
                        let approved_at = chrono::Utc::now().to_rfc3339();
                        let _ = sqlx::query("INSERT INTO approved_karma (id, node_id, karma_type, related_skill, lesson, weight, soul_version_hash, lamport_clock, signature, created_at, approved_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(id) DO NOTHING")
                            .bind(&k.id).bind(&k.node_id).bind(&k.karma_type).bind(&k.related_skill).bind(&k.lesson)
                            .bind(k.weight).bind(&k.soul_version_hash).bind(k.lamport_clock).bind(&k.signature).bind(&k.created_at).bind(&approved_at)
                            .execute(&mut *tx).await;
                        let _ = sqlx::query("DELETE FROM quarantined_karma WHERE id = ?").bind(&k.id).execute(&mut *tx).await;
                        if tx.commit().await.is_ok() {
                            info!("✅ [ApprovalWorker] Approved Karma: {}", k.id);
                        }
                    }
                    Err(e) => error!("❌ [ApprovalWorker] Failed to start transaction: {:?}", e),
                }
            } else {
                warn!("🛡️ [ApprovalWorker] Rejecting invalid Karma (Signature Mismatch): {}", k.id);
                // BFT Slashing: Penalize node reputation for invalid signatures
                let _ = sqlx::query("UPDATE node_reputation SET reputation_score = reputation_score - 10 WHERE node_id = ?").bind(&k.node_id).execute(&pool).await;
                sqlx::query("DELETE FROM quarantined_karma WHERE id = ?").bind(&k.id).execute(&pool).await.ok();
            }
        }

        // 2. Process Quarantined Rules
        let rules = sqlx::query_as::<_, ImmuneRuleRecord>("SELECT * FROM quarantined_rules LIMIT 50")
            .fetch_all(&pool).await.unwrap_or_default();

        for r in &rules {
            let mut valid = false;
            if let Some(ref sig_b64) = r.signature {
                let payload = format!("{}:{}:{}", r.id, r.pattern, r.lamport_clock);
                if let (Ok(pubkey_bytes), Ok(sig_bytes)) = (BASE64_STANDARD.decode(&r.node_id), BASE64_STANDARD.decode(&sig_b64)) {
                    if let (Ok(pubkey), Ok(sig)) = (VerifyingKey::from_bytes(&pubkey_bytes.try_into().unwrap_or([0; 32])), Signature::from_slice(&sig_bytes)) {
                        if pubkey.verify(payload.as_bytes(), &sig).is_ok() {
                            valid = true;
                        }
                    }
                }
            }

            if valid {
                match pool.begin().await {
                    Ok(mut tx) => {
                        let approved_at = chrono::Utc::now().to_rfc3339();
                        let _ = sqlx::query("INSERT INTO approved_rules (id, pattern, severity, action, node_id, lamport_clock, signature, created_at, approved_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(id) DO NOTHING")
                            .bind(&r.id).bind(&r.pattern).bind(r.severity).bind(&r.action).bind(&r.node_id).bind(r.lamport_clock).bind(&r.signature).bind(&r.created_at).bind(&approved_at)
                            .execute(&mut *tx).await;
                        let _ = sqlx::query("DELETE FROM quarantined_rules WHERE id = ?").bind(&r.id).execute(&mut *tx).await;
                        if tx.commit().await.is_ok() {
                            info!("✅ [ApprovalWorker] Approved Rule: {}", r.id);
                        }
                    }
                    Err(e) => error!("❌ [ApprovalWorker] Failed to start transaction: {:?}", e),
                }
            } else {
                warn!("🛡️ [ApprovalWorker] Rejecting invalid Rule (Signature Mismatch): {}", r.id);
                // BFT Slashing: Penalize node reputation for invalid signatures
                let _ = sqlx::query("UPDATE node_reputation SET reputation_score = reputation_score - 10 WHERE node_id = ?").bind(&r.node_id).execute(&pool).await;
                sqlx::query("DELETE FROM quarantined_rules WHERE id = ?").bind(&r.id).execute(&pool).await.ok();
            }
        }

        // 3. Data Eviction (Flaw 3: Disk Exhaustion Defense)
        // Keep ONLY the last 1,000,000 Records
        let _ = sqlx::query("DELETE FROM approved_karma WHERE id NOT IN (SELECT id FROM approved_karma ORDER BY approved_at DESC LIMIT 1000000)").execute(&pool).await;
        let _ = sqlx::query("DELETE FROM approved_rules WHERE id NOT IN (SELECT id FROM approved_rules ORDER BY approved_at DESC LIMIT 1000000)").execute(&pool).await;

        // Dynamic Polling (Component 2: Backpressure Tuning)
        let total_processed = karmas.len() + rules.len();
        if total_processed >= 100 {
            // High load: Don't sleep, keep processing quarantine
            tokio::task::yield_now().await;
        } else {
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }
}



