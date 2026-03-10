/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use axum::{
    response::Json,
    extract::State,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use crate::AppState;
use tracing::info;

#[derive(Serialize)]
pub struct SkillSummary {
    pub name: String,
    pub description: String,
    pub source: String, // "wasm", "mcp", "script"
    pub layer: u8,
}

#[derive(Deserialize)]
pub struct ImportRequest {
    pub url: String,
}

#[derive(Deserialize)]
pub struct McpSpawnRequest {
    pub id: String,
    pub command: String,
    pub args: Vec<String>,
}

/// [A-3] Skill Management Console API
/// Provides endpoints to list, import, and manage skills in the Swarm.
pub async fn list_skills(
    State(state): State<AppState>,
) -> Json<Vec<SkillSummary>> {
    let mut skills = Vec::new();

    // 1. Wasm Skills
    let wasm_meta = state.wasm_skill_manager.list_skills_with_metadata();
    for meta in wasm_meta {
        skills.push(SkillSummary {
            name: meta.name,
            description: meta.description,
            source: "wasm".to_string(),
            layer: 3,
        });
    }

    // 2. MCP Skills
    // Note: We'll list tools from currently connected MCP sessions
    // For now, static listing based on connected IDs.
    // In production, we'd list_tools() for each.
    Json(skills)
}

pub async fn import_skill(
    State(_state): State<AppState>,
    Json(payload): Json<ImportRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    info!("👹 [Vampire Attack] Attempting to import skill from: {}", payload.url);
    // Phase 20 MVP: Manual import trigger.
    // In production, this would use importer::parse_openapi() or similar.
    Ok(Json(serde_json::json!({"status": "queued", "message": "Import process initiated in cleanroom sandbox."})))
}

pub async fn spawn_mcp_server(
    State(state): State<AppState>,
    Json(payload): Json<McpSpawnRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.mcp_manager.spawn_stdio_server(payload.id.clone(), &payload.command, payload.args).await {
        Ok(_) => Ok(Json(serde_json::json!({"status": "success", "id": payload.id}))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))
    }
}
