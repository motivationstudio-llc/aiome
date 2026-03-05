/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

// ===== Core Connectivity State (Circuit Breaker) =====

#[derive(Debug, Clone)]
pub struct CoreState {
    pub is_online: Arc<RwLock<bool>>,
    pub base_url: String,
    pub client: reqwest::Client,
}

impl CoreState {
    pub fn new(base_url: &str) -> Self {
        Self {
            is_online: Arc::new(RwLock::new(false)),
            base_url: base_url.to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Check if Core API is reachable
    async fn health_check(&self) -> bool {
        match self.client.get(format!("{}/api/health", self.base_url)).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Pre-flight check: return error immediately if Core is offline
    async fn ensure_online(&self) -> Result<(), String> {
        let online = *self.is_online.read().await;
        if !online {
            return Err("Core is offline. Cannot process request.".to_string());
        }
        Ok(())
    }
}

// ===== API Response Types =====

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub id: String,
    pub title: String,
    pub style: Option<String>,
    pub created_at: String,
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RemixRequest {
    pub category: String,
    pub topic: String,
    pub remix_id: String,
    pub style_name: String,
    pub custom_style: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RemixResponse {
    pub job_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemStatus {
    pub cpu_usage: f64,
    pub memory_usage_mb: u64,
    pub vram_usage_mb: u64,
    pub active_actor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoreHealthStatus {
    pub online: bool,
}

// ===== Tauri Commands =====

/// Circuit Breaker: Check Core connectivity
#[tauri::command]
async fn get_core_status(state: State<'_, CoreState>) -> Result<CoreHealthStatus, String> {
    let online = *state.is_online.read().await;
    Ok(CoreHealthStatus { online })
}

/// Fetch all projects from the Warehouse
#[tauri::command]
async fn get_projects(state: State<'_, CoreState>) -> Result<Vec<ProjectSummary>, String> {
    state.ensure_online().await?;
    let resp = state.client
        .get(format!("{}/api/projects", state.base_url))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Core returned status {}", resp.status()));
    }

    resp.json::<Vec<ProjectSummary>>()
        .await
        .map_err(|e| format!("Failed to parse projects: {}", e))
}

/// Fetch available styles
#[tauri::command]
async fn get_styles(state: State<'_, CoreState>) -> Result<Vec<String>, String> {
    state.ensure_online().await?;
    let resp = state.client
        .get(format!("{}/api/styles", state.base_url))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Core returned status {}", resp.status()));
    }

    resp.json::<Vec<String>>()
        .await
        .map_err(|e| format!("Failed to parse styles: {}", e))
}

/// Submit a remix job
#[tauri::command]
async fn post_remix(state: State<'_, CoreState>, request: RemixRequest) -> Result<RemixResponse, String> {
    state.ensure_online().await?;
    let resp = state.client
        .post(format!("{}/api/remix", state.base_url))
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if resp.status().as_u16() == 429 {
        return Err("System busy! Request rejected (429).".to_string());
    }

    if !resp.status().is_success() {
        return Err(format!("Core returned status {}", resp.status()));
    }

    resp.json::<RemixResponse>()
        .await
        .map_err(|e| format!("Failed to parse remix response: {}", e))
}

/// Get asset URL (proxy for CORS-free access)
#[tauri::command]
async fn get_asset_url(state: State<'_, CoreState>, project_id: String, filename: String) -> Result<String, String> {
    Ok(format!("{}/assets/{}/{}", state.base_url, project_id, filename))
}

// ===== Application Entry Point =====

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let core_state = CoreState::new("http://127.0.0.1:3000");

    // Background health check poller
    let health_state = core_state.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(async move {
            loop {
                let is_up = health_state.health_check().await;
                let mut online = health_state.is_online.write().await;
                if *online != is_up {
                    if is_up {
                        eprintln!("🟢 [Tauri] Core API is online");
                    } else {
                        eprintln!("🔴 [Tauri] Core API is offline");
                    }
                }
                *online = is_up;
                drop(online);
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            }
        });
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(core_state)
        .invoke_handler(tauri::generate_handler![
            get_core_status,
            get_projects,
            get_styles,
            post_remix,
            get_asset_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
