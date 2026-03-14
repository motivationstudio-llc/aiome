/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

use crate::error::AppError;
use crate::AppState;
use aiome_core::traits::JobQueue;
use axum::{
    extract::Path, extract::State, http::StatusCode, response::IntoResponse, response::Json,
    routing::get,
};
use shared::health::{HealthMonitor, ResourceStatus};
use std::fs;

#[utoipa::path(
    get,
    path = "/api/wiki",
    responses(
        (status = 200, description = "List wiki markdown files", body = [String])
    ),
    security(("api_key" = []))
)]
pub async fn list_wiki_files(
    State(state): State<AppState>,
) -> Result<Json<Vec<String>>, AppError> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(&state.docs_path) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".md") {
                    files.push(name.to_string());
                }
            }
        }
    }
    files.sort();
    Ok(Json(files))
}

#[utoipa::path(
    get,
    path = "/api/wiki/{filename}",
    params(
        ("filename" = String, Path, description = "Filename with .md extension")
    ),
    responses(
        (status = 200, description = "Wiki markdown content", body = String),
        (status = 404, description = "File not found")
    ),
    security(("api_key" = []))
)]
pub async fn get_wiki_content(
    State(state): State<AppState>,
    Path(filename): Path<String>,
) -> Result<String, AppError> {
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: "Invalid filename".to_string(),
        }
        .into());
    }

    let path = std::path::PathBuf::from(&state.docs_path).join(filename);
    fs::read_to_string(path).map_err(|e| {
        aiome_core::error::AiomeError::OsError {
            source: e.into(),
        }
        .into()
    })
}

#[utoipa::path(
    get,
    path = "/api/health",
    responses(
        (status = 200, description = "Get current system and agent health status", body = ResourceStatus),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = []))
)]
pub async fn get_health_status(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
) -> Result<Json<ResourceStatus>, AppError> {
    let mut monitor = state.health_monitor.lock().await;
    let mut status = monitor.check();

    // Fetch real agent stats
    if let Ok(stats) = state.job_queue.get_agent_stats().await {
        status.level = stats.level;
        status.exp = stats.exp;
        status.resonance = stats.resonance;
        status.creativity = stats.creativity;
        status.fatigue = stats.fatigue;
    }

    Ok(Json(status))
}

#[derive(serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct LogEntryResponse {
    pub id: i64,
    pub timestamp: Option<String>,
    pub level: String,
    pub target: String,
    pub message: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/logs",
    params(
        ("limit" = Option<i64>, Query, description = "Maximum number of logs to return")
    ),
    responses(
        (status = 200, description = "Fetch application logs", body = Vec<LogEntryResponse>)
    )
)]
pub async fn get_logs(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<LogEntryResponse>>, AppError> {
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(100);

    let pool = state.job_queue.get_pool();
    let rows = sqlx::query_as::<_, LogEntryResponse>(
        "SELECT id, timestamp, level, target, message FROM app_logs ORDER BY id DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await;

    let logs = rows.map_err(|e| aiome_core::error::AiomeError::Infrastructure {
        reason: format!("DB Error: {}", e),
    })?;

    Ok(Json(logs))
}
