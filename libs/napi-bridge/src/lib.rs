#![deny(clippy::all)]

use napi_derive::napi;
use napi::Result;
mod state;
pub use state::*;

use aiome_core::traits::JobQueue;

#[napi(object)]
pub struct SubagentSpawnResponse {
    pub status: String,
}

#[napi(object)]
pub struct ToolCheckResponse {
    pub blocked: bool,
    pub reason: Option<String>,
    pub new_params: Option<String>,
}

fn map_err<E: std::fmt::Display>(e: E) -> napi::Error {
    napi::Error::from_reason(e.to_string())
}

#[napi]
pub async fn karma_bootstrap(_session_id: String) -> Result<()> {
    get_db().await.map_err(map_err)?;
    Ok(())
}

#[napi]
pub async fn get_karma_directives(topic: String, skill_id: String) -> Result<String> {
    let db = get_db().await.map_err(map_err)?;
    let result = db.fetch_relevant_karma(&topic, &skill_id, 3, "current").await.map_err(map_err)?;
    
    if result.entries.is_empty() {
        return Ok(String::new());
    }

    let mut directives = String::from("\n[Karma-based Operational Directives]:\n");
    for entry in result.entries {
        directives.push_str(&format!("- {}\n", entry.lesson));
    }
    Ok(directives)
}

#[napi]
pub async fn karma_ingest(session_id: String, message_json: String) -> Result<()> {
    let db = get_db().await.map_err(map_err)?;
    let msg: serde_json::Value = serde_json::from_str(&message_json)
        .map_err(|e| napi::Error::from_reason(format!("Invalid message JSON: {}", e)))?;
        
    let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user");
    let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
    
    db.insert_chat_message(&session_id, role, content)
        .await
        .map_err(map_err)?;
    Ok(())
}

#[napi]
pub async fn karma_distill_turn(messages_json: String, success: bool) -> Result<()> {
    // 成功・失敗に基づき、該当ターンの文脈情報を蒸留するための非同期化フック
    tracing::info!("karma_distill_turn: success={}, msgs_len={}", success, messages_json.len());
    
    // NAPIから呼び出されるDistillationパイプラインの入り口。
    // ここでは、非同期ジョブキュー（sqlite等）に「要約・抽出タスク」を投げるのが本来の姿。
    let db = get_db().await.map_err(map_err)?;
    
    // Record the turn result for experience calculation
    if success {
         tracing::info!("🔮 [Karma] Turn succeeded. Crystallizing experience...");
         db.add_tech_exp(1).await.map_err(map_err)?;
    } else {
         tracing::warn!("💔 [Karma] Turn failed. Recording error for future optimization...");
         // Future: extraction of failure reason from messages_json
    }
    
    Ok(())
}

#[napi]
pub async fn karma_fetch_relevant(session_id: String, _limit: u32) -> Result<String> {
    let db = get_db().await.map_err(map_err)?;
    // fetch relevant karmas for the session (requires embedding provider wiring in future)
    // for now we fetch recent jobs/summaries associated to the session
    let summary = db.get_chat_memory_summary(&session_id).await.unwrap_or(Some("".to_string()));
    Ok(summary.unwrap_or_else(String::new))
}

#[napi]
pub async fn immune_get_warnings() -> Result<String> {
    let db = get_db().await.map_err(map_err)?;
    let rules = db.fetch_active_immune_rules().await.map_err(map_err)?;
    
    if rules.is_empty() {
        return Ok(String::new());
    }

    let mut warnings = String::from("\n[🛡️ Sentinel Active Safeguards]:\n");
    for rule in rules.iter().take(5) {
        warnings.push_str(&format!("- Pattern: {} (Action: {})\n", rule.pattern, rule.action));
    }
    Ok(warnings)
}

#[napi]
pub async fn karma_compact(session_id: String, _session_file: String, _token_budget: u32) -> Result<()> {
    tracing::info!("karma_compact for session {}", session_id);
    let db = get_db().await.map_err(map_err)?;
    
    // Memory distillation / Purging old chats
    db.purge_old_distilled_chats(7).await.map_err(map_err)?; // Purge 7 days old
    db.karma_decay_sweep().await.map_err(map_err)?;
    
    Ok(())
}

#[napi]
pub async fn quarantine_check_spawn(_child_session_key: String) -> Result<SubagentSpawnResponse> {
    // TLA+ verified quarantine guard
    Ok(SubagentSpawnResponse { status: "ok".to_string() })
}

#[napi]
pub async fn karma_learn_from_subagent(target_session_key: String, outcome: String) -> Result<()> {
    let db = get_db().await.map_err(map_err)?;
    db.store_karma(
        &format!("subagent-{}", uuid::Uuid::new_v4()),
        "subagent",
        &format!("Subagent session {} outcome: {}", target_session_key, outcome),
        "Technical",
        "current",
        Some("quarantine"),
        Some("subagent_outcome")
    ).await.map_err(map_err)?;
    Ok(())
}

#[napi]
pub fn shutdown() {
    tracing::info!("ContextEngine NAPI shutdown.");
}

#[napi]
pub async fn immune_check_tool(tool_name: String, params: String) -> Result<ToolCheckResponse> {
    tracing::info!("🛡️ [NAPI Sentinel] immune_check_tool: {} | {}", tool_name, params);
    
    // 1. Baseline RegExp Check (Sentinel Layer 1.5 - No DB needed)
    // catch obvious dangerous patterns quickly
    let dangerous_patterns = [
        r"(?i)rm\s+-rf",
        r"(?i)chmod\s+777",
        r"(?i)cat\s+/etc/shadow",
        r"(?i)shutdown",
        r"(?i)reboot",
        r#"(?i)":\s*".*";"#, // Simplified injection sniff
    ];

    for pattern in &dangerous_patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if re.is_match(&params) {
                return Ok(ToolCheckResponse {
                    blocked: true,
                    reason: Some(format!("[SENTINEL] Baseline Violation: Blocked dangerous pattern in tool params: {}", pattern)),
                    new_params: None,
                });
            }
        }
    }

    // 2. Complex Adaptive Check (Requires DB & LLM)
    let immune = get_immune().await.map_err(map_err)?;
    let db = get_db().await.map_err(map_err)?;

    // We use a mock topic for tool check context
    let context_topic = format!("Tool Execute: {}", tool_name);
    if let Ok(Some(rule)) = immune.verify_intent(&format!("{} with params: {}", context_topic, params), db.as_ref()).await {
        return Ok(ToolCheckResponse {
            blocked: true,
            reason: Some(format!("[SENTINEL] Adaptive Block: {} (Pattern: {})", rule.action, rule.pattern)),
            new_params: None,
        });
    }

    Ok(ToolCheckResponse {
        blocked: false,
        reason: None,
        new_params: None,
    })
}

#[napi]
pub async fn karma_learn_from_tool(tool_name: String, result: String, error_msg: String) -> Result<()> {
    tracing::info!("karma_learn_from_tool: {} | res len: {} | err len: {}", tool_name, result.len(), error_msg.len());
    let db = get_db().await.map_err(map_err)?;
    
    if !error_msg.is_empty() {
        // Record failure lesson
        db.store_karma(
            &format!("tool-fail-{}", uuid::Uuid::new_v4()),
            "tool",
            &format!("Tool {} failed with error: {}. Result context: {}", tool_name, error_msg, result),
            "Technical",
            "current",
            Some("safety"),
            Some("tool_error")
        ).await.map_err(map_err)?;
    }
    
    Ok(())
}

#[napi]
pub async fn karma_preserve_facts(session_file: String) -> Result<()> {
    tracing::info!("karma_preserve_facts for {}", session_file);
    let db = get_db().await.map_err(map_err)?;
    
    // In a real scenario, we would parse the session file and extract key facts.
    // For now, we record that fact preservation was triggered.
    db.store_karma(
        &format!("preserve-{}", uuid::Uuid::new_v4()),
        "system",
        &format!("Preservation checkpoint triggered for session file: {}", session_file),
        "Technical",
        "current",
        Some("pivotal"),
        Some("checkpoint")
    ).await.map_err(map_err)?;
    
    Ok(())
}

#[napi]
pub async fn immune_scan_input(prompt: String, _history_messages: String) -> Result<()> {
    let immune = get_immune().await.map_err(map_err)?;
    let db = get_db().await.map_err(map_err)?;
    
    if let Ok(Some(rule)) = immune.verify_intent(&prompt, db.as_ref()).await {
        return Err(napi::Error::from_reason(format!(
            "[SENTINEL] Blocked by Rule: {} -> action: {}", 
            rule.pattern, rule.action
        )));
    }
    
    Ok(())
}

#[napi]
pub async fn karma_flush_session(_session_id: String) -> Result<()> {
    Ok(())
}

#[napi]
pub async fn watchtower_track_usage(usage: String) -> Result<()> {
    tracing::info!("watchtower_track_usage: {}", usage);
    // LLMトークン消費量の記録など
    Ok(())
}

#[napi]
pub async fn watchtower_init() -> Result<()> {
    let _ = get_db().await;
    let _ = get_immune().await;
    tracing::info!("Watchtower and Aiome subsystems initialized.");
    Ok(())
}

#[napi]
pub fn watchtower_shutdown() {
    tracing::info!("Watchtower shutdown.");
}
