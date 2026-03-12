/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use axum::{
    response::Json,
    extract::{State, Query},
    http::StatusCode,
};
use crate::AppState;
use aiome_core::traits::JobQueue;
use aiome_core::expression::engine::ExpressionEngine;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ListParams {
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct AutoToggle {
    pub enabled: bool,
}

pub async fn expression_status(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let pending_count = state.job_queue.get_pending_job_count().await.unwrap_or(0);
    let auto_enabled = state.job_queue.get_auto_expression_enabled().await.unwrap_or(false);
    let recent_karma = state.job_queue.fetch_all_karma(1).await.unwrap_or_default();
    
    let status = if pending_count > 0 { "processing" } else { "idle" };
    let last_lesson = recent_karma.get(0).and_then(|k| k["lesson"].as_str()).unwrap_or("Waiting for new insights...");

    Json(serde_json::json!({
        "status": status,
        "auto_expression": auto_enabled,
        "pending_expressions": pending_count,
        "last_insight": last_lesson,
        "message_ja": format!("自律表現パイプライン: {} (自動: {})。現在の洞察: {}", status, if auto_enabled { "ON" } else { "OFF" }, last_lesson),
        "message_en": format!("Autonomous expression pipeline {} (Auto: {}). Current insight: {}", status, if auto_enabled { "ON" } else { "OFF" }, last_lesson)
    }))
}

pub async fn generate_expression(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // 1. Fetch latest Karma
    let karma = state.job_queue.fetch_all_karma(5).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;

    if karma.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "No karma available to generate expression"}))));
    }

    // 2. Fetch Soul Prompt
    let soul_prompt = state.soul_mutator.get_active_prompt().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;

    // 3. Generate Expression
    let expression = ExpressionEngine::generate(&karma, &soul_prompt, state.provider.as_ref()).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;

    // 4. Store Expression
    state.job_queue.store_expression(&expression).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;

    Ok(Json(serde_json::json!(expression)))
}

pub async fn list_expressions(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let limit = params.limit.unwrap_or(20);
    let expressions = state.job_queue.fetch_expressions(limit).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;

    Ok(Json(serde_json::json!(expressions)))
}

pub async fn toggle_auto_expression(
    State(state): State<AppState>,
    Json(payload): Json<AutoToggle>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.job_queue.set_auto_expression_enabled(payload.enabled).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "auto_expression_enabled": payload.enabled
    })))
}
