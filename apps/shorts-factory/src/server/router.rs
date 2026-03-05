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
    extract::{State, WebSocketUpgrade, ws::WebSocket},
    response::IntoResponse,
    routing::{get, post},
    Router, Json,
    http::{StatusCode, HeaderMap},
};
use std::sync::Arc;
use parking_lot::Mutex;
use crate::server::telemetry::TelemetryHub;
use crate::orchestrator::ProductionOrchestrator;
use factory_core::contracts::WorkflowRequest;
use factory_core::traits::{AgentAct, JobQueue}; // Trait import needed 
use tuning::StyleManager;
use bastion::fs_guard::Jail;
use tower_http::services::ServeDir;
use uuid::Uuid;
use crate::asset_manager::AssetManager;
use infrastructure::job_queue::SqliteJobQueue;

pub struct AppState {
    pub telemetry: Arc<TelemetryHub>,
    pub orchestrator: Arc<ProductionOrchestrator>,
    pub style_manager: Arc<StyleManager>,
    pub jail: Arc<Jail>,
    pub is_busy: Arc<Mutex<bool>>, // Resource Locking
    pub asset_manager: Arc<AssetManager>,
    pub current_job: Arc<tokio::sync::Mutex<Option<String>>>,
    pub job_queue: Arc<SqliteJobQueue>,
}


use tower_http::cors::CorsLayer;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/ws", get(websocket_handler))
        .route("/api/remix", post(remix_handler))
        .route("/api/styles", get(styles_handler))
        .route("/api/projects", get(projects_handler))
        .route("/api/jobs", get(jobs_handler))
        .route("/api/jobs/:id", get(job_detail_handler))
        .route("/api/jobs/:id/rate", post(job_rate_handler))
        .route("/api/karma", get(karma_handler))
        .route("/api/v1/federation/sync", post(federation_sync_handler))
        .nest_service("/assets", ServeDir::new("workspace")) // Serve static assets
        .layer(CorsLayer::permissive())
        .with_state(state)
}

// --- WebSocket Handler ---

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx_hb = state.telemetry.subscribe_heartbeat();
    let mut rx_log = state.telemetry.subscribe_log();

    loop {
        tokio::select! {
            Ok(hb) = rx_hb.recv() => {
                // Determine active actor based on busy state
                let mut hb_with_state = hb.clone();
                {
                    let busy = state.is_busy.lock();
                    if *busy {
                        hb_with_state.active_actor = Some("ORCHESTRATOR".to_string());
                    }
                } // Drop lock here before await

                if let Ok(msg) = serde_json::to_string(&hb_with_state) {
                    if socket.send(axum::extract::ws::Message::Text(msg)).await.is_err() {
                        break;
                    }
                }
            }
            Ok(log) = rx_log.recv() => {
                if let Ok(msg) = serde_json::to_string(&log) {
                    if socket.send(axum::extract::ws::Message::Text(msg)).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
}

// --- REST API Handlers ---

async fn remix_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<WorkflowRequest>,
) -> impl IntoResponse {
    // 1. Resource Locking (Overzealous Clicker Guard)
    {
        let mut busy = state.is_busy.lock();
        if *busy {
             state.telemetry.broadcast_log("WARN", "Rejecting concurrent remix request.");
             return (StatusCode::TOO_MANY_REQUESTS, Json(serde_json::json!({
                 "error": "System is busy. Please wait for the current task to finish."
             }))).into_response();
        }
        *busy = true; // Acquire lock
    }

    let job_id = Uuid::new_v4().to_string();
    state.telemetry.broadcast_log("INFO", &format!("Job Accepted: {} (Remix)", job_id));
    
    let orchestrator = state.orchestrator.clone();
    let jail = state.jail.clone();
    let busy_lock = state.is_busy.clone();
    let telemetry = state.telemetry.clone();
    let job_id_clone = job_id.clone();
    
    // 2. Asynchronous Job Creation
    let state_clone = state.clone();
    tokio::spawn(async move {
        // Set current job info
        {
            let mut job_info = state_clone.current_job.lock().await;
            *job_info = Some(format!("Remix: {}", job_id_clone));
        }

        // Execute the heavy task
        match orchestrator.execute(payload.clone(), &jail).await {
            Ok(res) => {
                let video_count = res.output_videos.len();
                let msg = format!("Job Completed: {} -> {} videos generated ({})", job_id_clone, video_count, res.final_video_path);
                println!("{}", msg);
                telemetry.broadcast_log("INFO", &msg);
            }
            Err(e) => {
                let msg = format!("Job Failed: {} -> {}", job_id_clone, e);
                eprintln!("{}", msg);
                telemetry.broadcast_log("ERROR", &msg);
            }
        }

        // Release Lock & Clear job info
        {
            let mut job_info = state_clone.current_job.lock().await;
            *job_info = None;
        }

        let mut busy = busy_lock.lock();
        *busy = false;
        telemetry.broadcast_log("INFO", "System Ready");
    });

    // 3. Immediate Response (202 Accepted)
    (StatusCode::ACCEPTED, Json(serde_json::json!({ 
        "status": "accepted", 
        "job_id": job_id,
        "job_type": "remix" 
    }))).into_response()
}

async fn styles_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let styles = state.style_manager.list_available_styles();
    Json(styles)
}

async fn projects_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let projects = state.asset_manager.list_projects();
    Json(projects)
}

// --- Job & Karma Handlers ---
use axum::extract::Path;

pub async fn jobs_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.job_queue.fetch_recent_jobs(100).await {
        Ok(jobs) => (StatusCode::OK, Json(serde_json::to_value(jobs).unwrap_or_default())).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

pub async fn job_detail_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    use factory_core::traits::JobQueue;
    match state.job_queue.fetch_job(&id).await {
        Ok(Some(job)) => (StatusCode::OK, Json(serde_json::to_value(job).unwrap_or_default())).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Job not found"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

pub async fn karma_handler(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match state.job_queue.fetch_all_karma(200).await {
        Ok(karmas) => (StatusCode::OK, Json(serde_json::to_value(karmas).unwrap_or_default())).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

pub async fn job_rate_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    use factory_core::traits::JobQueue;
    let rating = payload.get("rating").and_then(|v| v.as_i64()).unwrap_or(50) as i32;
    match state.job_queue.set_creative_rating(&id, rating).await {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({"status": "success"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

// --- Federation Handler ---

pub async fn federation_sync_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<factory_core::contracts::FederationSyncRequest>,
) -> impl IntoResponse {
    // 1. Authentication (The Auth Wall)
    let secret = std::env::var("FEDERATION_SECRET").unwrap_or_else(|_| "aiome_secret".to_string());
    let auth_header = headers.get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    match auth_header {
        Some(h) if h == format!("Bearer {}", secret) => {
            // Authorized
            use factory_core::traits::JobQueue;
            match state.job_queue.export_federated_data(payload.since.as_deref()).await {
                Ok((karmas, rules, matches)) => {
                    let response = factory_core::contracts::FederationSyncResponse {
                        new_karmas: karmas,
                        new_immune_rules: rules,
                        new_arena_matches: matches,
                        server_time: chrono::Utc::now().to_rfc3339(),
                    };
                    (StatusCode::OK, Json(response)).into_response()
                }
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
            }
        }
        _ => {
            tracing::warn!("🔒 [Federation] Unauthorized sync attempt blocked");
            (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "Unauthorized"}))).into_response()
        }
    }
}
