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
    pub source: String, // "wasm", "mcp", "marketplace"
    pub status: String, // "Active", "Installed", "Available"
    pub layer: u8,
    pub tools: Vec<String>,
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
            name: meta.name.clone(),
            description: meta.description,
            source: "wasm".to_string(),
            status: "Active".to_string(),
            layer: 3,
            tools: meta.capabilities,
        });
    }

    // 2. MCP Skills (Active Clients)
    let mcp_ids = state.mcp_manager.active_client_ids().await;
    for id in mcp_ids {
        if let Some(client) = state.mcp_manager.get_client(&id).await {
            let tools = client.list_tools().await.unwrap_or_default();
            skills.push(SkillSummary {
                name: id.clone(),
                description: format!("Running MCP Server: {}", id),
                source: "mcp".to_string(),
                status: "Active".to_string(),
                layer: 4,
                tools: tools.into_iter().map(|t| t.name).collect(),
            });
        }
    }

    // 3. Mock Marketplace Skills (Discovery Phase 2B DEMO)
    skills.push(SkillSummary {
        name: "Browser Automation".to_string(),
        description: "Control a headless browser to scrape data or interact with sites.".to_string(),
        source: "marketplace".to_string(),
        status: "Available".to_string(),
        layer: 4,
        tools: vec!["click_element".to_string(), "navigate".to_string(), "screenshot".to_string()],
    });
    skills.push(SkillSummary {
        name: "Financial Analyst".to_string(),
        description: "Real-time stock market data and financial report analysis.".to_string(),
        source: "marketplace".to_string(),
        status: "Available".to_string(),
        layer: 5,
        tools: vec!["get_stock_price".to_string(), "analyze_trend".to_string()],
    });

    Json(skills)
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
