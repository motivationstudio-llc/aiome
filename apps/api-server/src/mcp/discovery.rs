/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::mcp::client::McpProcessManager;
use tracing::{info, error};

#[derive(Debug, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpDiscoveryFile {
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

/// [A-3] MCP Discovery Layer
/// Scans local configuration to automatically connect to external MCP tools.
pub async fn discover_and_spawn(manager: &McpProcessManager) -> anyhow::Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let config_path = PathBuf::from(home).join(".aiome/mcp_servers.json");

    if !config_path.exists() {
        info!("ℹ️ [MCP Discovery] No server config found at ~/.aiome/mcp_servers.json");
        return Ok(());
    }

    let content = std::fs::read_to_string(&config_path)?;
    let discovery: McpDiscoveryFile = serde_json::from_str(&content)?;

    for (id, config) in discovery.mcp_servers {
        info!("🔍 [MCP Discovery] Found registered server: {}", id);
        // Spawn each server in the background
        if let Err(e) = manager.spawn_stdio_server(id.clone(), &config.command, config.args).await {
            error!("🚨 [MCP Discovery] Failed to spawn {}: {}", id, e);
        }
    }

    Ok(())
}
