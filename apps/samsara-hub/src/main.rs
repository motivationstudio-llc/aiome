/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

use axum::{
    extract::State,
    routing::{get, post},
    response::{self, IntoResponse},
    Router, Json,
    http::{StatusCode, HeaderMap},
};
use std::sync::Arc;
use sqlx::{SqlitePool, Row};
use factory_core::contracts::{FederationSyncRequest, FederationSyncResponse, FederationPushRequest, FederationPushResponse, FederatedKarma, ImmuneRule};
use tracing::{info, warn, error};
use tower_http::cors::CorsLayer;

struct HubState {
    pool: SqlitePool,
    secret: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing with JSON for easier aggregation in the hub
    tracing_subscriber::fmt().json().init();
    
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:samsara_hub.db?mode=rwc".to_string());
    let secret = std::env::var("FEDERATION_SECRET").unwrap_or_else(|_| "hub_secret_1337".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3016".to_string());

    let pool = SqlitePool::connect(&db_url).await?;
    init_hub_db(&pool).await?;

    let state = Arc::new(HubState { pool, secret });

    let app = Router::new()
        .route("/api/v1/federation/sync", post(sync_handler))
        .route("/api/v1/federation/push", post(push_handler))
        .route("/api/v1/health", get(health_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
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
            node_id TEXT,
            karma_type TEXT NOT NULL,
            related_skill TEXT NOT NULL,
            lesson TEXT NOT NULL,
            weight INTEGER NOT NULL,
            soul_version_hash TEXT,
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
    // Auth Wall
    let auth = headers.get(axum::http::header::AUTHORIZATION).and_then(|h| h.to_str().ok());
    if auth != Some(&format!("Bearer {}", state.secret)) {
        warn!("🔒 Unauthorized sync attempt from node: {}", payload.node_id);
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))).into_response();
    }

    info!("🌐 Node {} pulling approved updates since {:?}", payload.node_id, payload.since);

    let since = payload.since.unwrap_or_else(|| "1970-01-01T00:00:00".to_string());

    // Fetch ONLY approved data
    let karmas = sqlx::query_as::<_, FederatedKarmaRecord>(
        "SELECT id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at FROM approved_karma WHERE approved_at > ?"
    ).bind(&since).fetch_all(&state.pool).await.unwrap_or_default();

    let rules = sqlx::query_as::<_, ImmuneRuleRecord>(
        "SELECT id, pattern, severity, action, created_at FROM approved_rules WHERE approved_at > ?"
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
        }).collect(),
        new_immune_rules: rules.into_iter().map(|r| ImmuneRule {
            id: r.id,
            pattern: r.pattern,
            severity: r.severity as u8,
            action: r.action,
            created_at: r.created_at,
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
    let auth = headers.get(axum::http::header::AUTHORIZATION).and_then(|h| h.to_str().ok());
    if auth != Some(&format!("Bearer {}", state.secret)) {
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

    for k in payload.karmas {
        let _ = sqlx::query(
            "INSERT INTO quarantined_karma (id, node_id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO NOTHING"
        )
        .bind(&k.id).bind(&payload.node_id).bind(&k.karma_type).bind(&k.related_skill).bind(&k.lesson)
        .bind(k.weight as i64).bind(&k.soul_version_hash).bind(&k.created_at)
        .execute(&mut *tx).await;
    }

    for r in payload.rules {
        let _ = sqlx::query(
            "INSERT INTO quarantined_rules (id, node_id, pattern, severity, action, created_at)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO NOTHING"
        )
        .bind(&r.id).bind(&payload.node_id).bind(&r.pattern).bind(r.severity as i64).bind(&r.action).bind(&r.created_at)
        .execute(&mut *tx).await;
    }

    if let Err(e) = tx.commit().await {
        error!("❌ Push commit failed: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }

    (StatusCode::OK, Json(FederationPushResponse {
        accepted_count: karma_count + rule_count,
        message: "Data received and placed in quarantine for validation.".to_string(),
    })).into_response()
}

// Helper structs for query_as (sqlx requirement)
#[derive(sqlx::FromRow)]
struct FederatedKarmaRecord {
    id: String,
    karma_type: String,
    related_skill: String,
    lesson: String,
    weight: i64,
    soul_version_hash: Option<String>,
    created_at: String,
}

#[derive(sqlx::FromRow)]
struct ImmuneRuleRecord {
    id: String,
    pattern: String,
    severity: i64,
    action: String,
    created_at: String,
}
