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
use aiome_core::expression::engine::ExpressionEngine;
use aiome_core::traits::JobQueue;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ListParams {
    pub limit: Option<i64>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct AutoToggle {
    pub enabled: bool,
}

#[utoipa::path(
    get,
    path = "/api/expression/status",
    responses(
        (status = 200, description = "Expression engine status", body = serde_json::Value)
    )
)]
pub async fn expression_status(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pending_count = state.job_queue.get_pending_job_count().await.unwrap_or(0);
    let auto_enabled = state
        .job_queue
        .get_auto_expression_enabled()
        .await
        .unwrap_or(false);
    let recent_karma = state.job_queue.fetch_all_karma(1).await.unwrap_or_default();

    let status = if pending_count > 0 {
        "processing"
    } else {
        "idle"
    };
    let last_lesson = recent_karma
        .get(0)
        .and_then(|k| k["lesson"].as_str())
        .unwrap_or("Waiting for new insights...");

    Ok(Json(serde_json::json!({
        "status": status,
        "auto_expression": auto_enabled,
        "pending_expressions": pending_count,
        "last_insight": last_lesson,
        "message_ja": format!("自律表現パイプライン: {} (自動: {})。現在の洞察: {}", status, if auto_enabled { "ON" } else { "OFF" }, last_lesson),
        "message_en": format!("Autonomous expression pipeline {} (Auto: {}). Current insight: {}", status, if auto_enabled { "ON" } else { "OFF" }, last_lesson)
    })))
}

#[utoipa::path(
    post,
    path = "/api/expression/generate",
    responses(
        (status = 200, description = "Generated expression", body = serde_json::Value),
        (status = 400, description = "No karma available")
    )
)]
pub async fn generate_expression(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    // 1. Fetch latest Karma
    let karma = state.job_queue.fetch_all_karma(5).await?;

    if karma.is_empty() {
        return Err(aiome_core::error::AiomeError::Infrastructure {
            reason: "No karma available to generate expression".to_string(),
        }
        .into());
    }

    // 2. Fetch Soul Prompt
    let soul_prompt = state.soul_mutator.get_active_prompt().await?;

    // 3. Generate Expression
    let expression =
        ExpressionEngine::generate(&karma, &soul_prompt, state.provider.as_ref()).await?;

    // 4. Store Expression
    state.job_queue.store_expression(&expression).await?;

    Ok(Json(serde_json::json!(expression)))
}

#[utoipa::path(
    get,
    path = "/api/expression/list",
    params(
        ("limit" = Option<i64>, Query, description = "Limit results")
    ),
    responses(
        (status = 200, description = "Recent expressions", body = [serde_json::Value])
    )
)]
pub async fn list_expressions(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let limit = params.limit.unwrap_or(20);
    let expressions = state.job_queue.fetch_expressions(limit).await?;

    Ok(Json(serde_json::json!(expressions)))
}

#[utoipa::path(
    post,
    path = "/api/expression/auto",
    request_body = AutoToggle,
    responses(
        (status = 200, description = "Toggled auto-expression", body = serde_json::Value)
    )
)]
pub async fn toggle_auto_expression(
    State(state): State<AppState>,
    Json(payload): Json<AutoToggle>,
) -> Result<Json<serde_json::Value>, AppError> {
    state
        .job_queue
        .set_auto_expression_enabled(payload.enabled)
        .await?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "auto_expression_enabled": payload.enabled
    })))
}
