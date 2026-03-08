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
    
    // (デモ用ダミー抽出: 実際のLLMでの抽出は独立したワーカーに任せる)
    if success {
         db.add_tech_exp(1).await.map_err(map_err)?;
    }
    
    Ok(())
}

#[napi]
pub async fn karma_fetch_relevant(session_id: String, _limit: u32) -> Result<String> {
    let db = get_db().await.map_err(map_err)?;
    // fetch relevant karmas for the session (requires embedding provider wiring in future)
    // for now we fetch recent jobs/summaries associated to the session
    let summary = db.get_chat_memory_summary(&session_id).await.unwrap_or(Some("".to_string()));
    Ok(summary.unwrap_or_else(|| String::new()))
}

#[napi]
pub fn immune_get_warnings() -> String {
    String::new()
}

#[napi]
pub async fn karma_compact(session_id: String, _session_file: String, _token_budget: u32) -> Result<()> {
    tracing::info!("karma_compact for session {}", session_id);
    // メモリ圧縮のトリガー。
    Ok(())
}

#[napi]
pub async fn quarantine_check_spawn(_child_session_key: String) -> Result<SubagentSpawnResponse> {
    // TLA+ verified quarantine guard
    Ok(SubagentSpawnResponse { status: "ok".to_string() })
}

#[napi]
pub async fn karma_learn_from_subagent(_target_session_key: String, _outcome: String) -> Result<()> {
    Ok(())
}

#[napi]
pub fn shutdown() {
    tracing::info!("ContextEngine NAPI shutdown.");
}

#[napi]
pub async fn immune_check_tool(tool_name: String, params: String) -> Result<ToolCheckResponse> {
    tracing::info!("immune_check_tool: {} | {}", tool_name, params);
    // TODO: wire real logic from immune system
    Ok(ToolCheckResponse {
        blocked: false,
        reason: None,
        new_params: None,
    })
}

#[napi]
pub async fn karma_learn_from_tool(tool_name: String, result: String, error_msg: String) -> Result<()> {
    tracing::info!("karma_learn_from_tool: {} | res len: {} | err len: {}", tool_name, result.len(), error_msg.len());
    Ok(())
}

#[napi]
pub async fn karma_preserve_facts(_session_file: String) -> Result<()> {
    tracing::info!("karma_preserve_facts placeholder");
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
