/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

use crate::mcp::types::{JsonRpcRequest, JsonRpcResponse};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{oneshot, Mutex};
use tracing::{info, warn};

/// Phase 17-B: Zombie Defense - Managed child process.
/// It uses a background task to handle JSON-RPC multiplexing.
pub struct McpClient {
    pub id: String,
    stdin: Arc<Mutex<tokio::process::ChildStdin>>,
    pending_requests: Arc<Mutex<HashMap<i64, oneshot::Sender<JsonRpcResponse>>>>,
    request_counter: AtomicI64,
}

impl McpClient {
    pub fn spawn(id: String, cmd: &str, args: Vec<String>) -> Result<Arc<Self>> {
        info!(
            "🚀 [MCP] Spawning stdio server: {} for session: {}",
            cmd, id
        );

        // Use tokio::process::Command for async I/O
        let mut child = Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true) // Defense against zombie processes
            .spawn()?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Failed to open stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Failed to open stdout"))?;
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow!("Failed to open stderr"))?;

        let pending_requests = Arc::new(Mutex::new(
            HashMap::<i64, oneshot::Sender<JsonRpcResponse>>::new(),
        ));
        let pending_requests_clone = pending_requests.clone();
        let client_id = id.clone();

        // Stderr logging task
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                warn!("⚠️ [MCP:{}] stderr: {}", client_id, line);
            }
        });

        // Stdout JSON-RPC parser task
        let pending_requests_for_stdout = pending_requests.clone();
        let client_id_for_stdout = id.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if let Ok(response) = serde_json::from_str::<JsonRpcResponse>(&line) {
                    if let Some(id_val) = response.id.as_i64() {
                        let mut reqs = pending_requests_for_stdout.lock().await;
                        if let Some(tx) = reqs.remove(&id_val) {
                            let _ = tx.send(response);
                        }
                    }
                } else {
                    info!("📖 [MCP:{}] raw line: {}", client_id_for_stdout, line);
                }
            }
            info!(
                "🔌 [MCP:{}] stdout task ended (connection closed)",
                client_id_for_stdout
            );
        });

        Ok(Arc::new(Self {
            id,
            stdin: Arc::new(Mutex::new(stdin)),
            pending_requests,
            request_counter: AtomicI64::new(1),
        }))
    }

    pub async fn call(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let id = self.request_counter.fetch_add(1, Ordering::SeqCst);
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: Some(serde_json::json!(id)),
        };

        let (tx, rx) = oneshot::channel();
        {
            let mut reqs = self.pending_requests.lock().await;
            reqs.insert(id, tx);
        }

        let mut stdin = self.stdin.lock().await;
        let json_req = serde_json::to_string(&request)? + "\n";
        stdin.write_all(json_req.as_bytes()).await?;
        stdin.flush().await?;

        // Wait for response
        let response = rx
            .await
            .map_err(|_| anyhow!("MCP connection closed before response"))?;

        if let Some(error) = response.error {
            return Err(anyhow!("MCP Error ({}): {}", error.code, error.message));
        }

        response
            .result
            .ok_or_else(|| anyhow!("Empty result from MCP"))
    }

    // High level MCP methods
    pub async fn list_tools(&self) -> Result<Vec<crate::mcp::types::McpTool>> {
        let res = self.call("tools/list", None).await?;
        let list: crate::mcp::types::ListToolsResult = serde_json::from_value(res)?;
        Ok(list.tools)
    }

    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<crate::mcp::types::CallToolResult> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments
        });
        let res = self.call("tools/call", Some(params)).await?;
        Ok(serde_json::from_value(res)?)
    }
}

pub struct McpProcessManager {
    clients: Arc<Mutex<HashMap<String, Arc<McpClient>>>>,
}

impl McpProcessManager {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn get_client(&self, id: &str) -> Option<Arc<McpClient>> {
        let clients = self.clients.lock().await;
        clients.get(id).cloned()
    }

    pub async fn spawn_stdio_server(
        &self,
        id: String,
        cmd: &str,
        args: Vec<String>,
    ) -> Result<Arc<McpClient>> {
        let client = McpClient::spawn(id.clone(), cmd, args)?;
        let mut clients = self.clients.lock().await;
        clients.insert(id, client.clone());
        Ok(client)
    }

    pub async fn active_client_ids(&self) -> Vec<String> {
        let clients = self.clients.lock().await;
        clients.keys().cloned().collect()
    }

    pub async fn kill_all(&self) {
        let mut clients = self.clients.lock().await;
        info!("💥 [MCP] Evicting {} managed MCP clients", clients.len());
        clients.clear();
    }
}
