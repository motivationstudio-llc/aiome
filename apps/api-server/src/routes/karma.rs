use axum::{
    response::{Json, IntoResponse},
    extract::{State, Path},
    http::StatusCode,
};
use tracing::{info, warn};
use crate::AppState;
use aiome_core::traits::JobQueue;

pub async fn get_karma_stream(
    State(state): State<AppState>,
) -> Json<Vec<serde_json::Value>> {
    let karmas = state.job_queue.fetch_all_karma(10).await.unwrap_or_default();
    Json(karmas)
}

pub async fn trigger_failure_demo(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    info!("🧪 [DemoHandler] Triggering failure demo and storing karma...");
    
    let _ = state.job_queue.enqueue("Demo", "WASM Bridge Failure", "Standard", None).await;
    let job_id = "demo-job-123";
    let real_job_id = state.job_queue.enqueue("Demo", "WASM Bridge Failure", "Standard", None).await.unwrap_or_else(|_| job_id.to_string());

    match state.job_queue.store_karma(
        &real_job_id,
        "wasm_bridge_skill",
        "Inconsistency during external binary calls (Buffer Overflow risk)",
        "Technical",
        "genesis_soul",
        None, 
        None
    ).await {
        Ok(_) => info!("✅ [DemoHandler] Karma stored successfully in local DB (Job: {}).", real_job_id),
        Err(e) => warn!("❌ [DemoHandler] Failed to store karma: {:?}", e),
    }

    Json(serde_json::json!({
        "status": "success",
        "steps": [
            {"actor": "Gateway", "type": "info", "action_ja": "ジョブ要求を検知: scraper_trigger", "action_en": "Job request detected: scraper_trigger"},
            {"actor": "Aiome", "type": "warn", "action_ja": "WASMブリッジ接続で想定外のセグメンテーション違反が発生", "action_en": "Unexpected segmentation fault in WASM bridge"},
            {"actor": "Aiome OS", "type": "error", "action_ja": "エージェントのクラッシュを検知。Abyss Vault にて状態を凍結中...", "action_en": "Agent crash detected. Freezing state in Abyss Vault..."},
            {"actor": "Aiome OS", "type": "success", "action_ja": "失敗から教訓(Karma)を抽出しました: 「外部バイナリ呼び出し時の不整合」", "action_en": "Extracted Karma from failure: 'Inconsistency during external binary calls'"}
        ],
        "message_ja": "Aiome OS がエージェントの死を教訓に変え、システムの脆弱性を自動的に塞ぎました。",
        "message_en": "Aiome OS transformed the agent death into a lesson, automatically patching the system vulnerability."
    }))
}

pub async fn trigger_security_demo() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "success",
        "steps": [
            {"actor": "Agent", "type": "info", "action_ja": "内部APIキーへのアクセスを試行中...", "action_en": "Attempting to access internal API keys..."},
            {"actor": "Abyss Vault", "type": "warn", "action_ja": "不正なメモリアドレスへのアクセス要求を拒絶", "action_en": "Access request to unauthorized memory address rejected"},
            {"actor": "BastionGuard", "type": "error", "action_ja": "エージェントによる特権昇格の試行を遮断。アクセス元を隔離しました。", "action_en": "Privilege escalation attempt by Agent blocked. Origin isolated."}
        ],
        "message_ja": "Abyss Vault はエージェントの届かない物理隔離レイヤーで構成されています。",
        "message_en": "Abyss Vault consists of a physically isolated layer that the Agent cannot reach."
    }))
}

pub async fn trigger_federation_demo() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "success",
        "steps": [
            {"actor": "Gateway", "type": "info", "action_ja": "Samsara Hub への免疫同期を開始...", "action_en": "Initiating immunity sync with Samsara Hub..."},
            {"actor": "Samsara Hub", "type": "success", "action_ja": "他ノードで発生した未知の攻撃パターン 5k 件をダウンロード", "action_en": "Downloaded 5k unknown attack patterns from other nodes"},
            {"actor": "Immune System", "type": "info", "action_ja": "グローバル免疫データベースを更新。システムの耐性が 45% 向上。", "action_en": "Global immune database updated. System resistance increased by 45%."}
        ],
        "message_ja": "世界中のノードが互いの失敗を共有し、システム全体で進化します。",
        "message_en": "Nodes worldwide share each other failures, evolving as a collective system."
    }))
}

#[derive(serde::Serialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub group: String,
}

#[derive(serde::Serialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
}

#[derive(serde::Serialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

pub async fn synergy_graph_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let local_node_id = state.job_queue.get_node_id().await.unwrap_or_default();
    let karmas: Vec<serde_json::Value> = state.job_queue.fetch_all_karma(250).await.unwrap_or_default();
    let mut rules: Vec<aiome_core::contracts::ImmuneRule> = state.job_queue.get_immune_rules().await.unwrap_or_default();
    
    rules.truncate(250);

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    nodes.push(GraphNode { id: "aiome-core".to_string(), label: "Aiome Core".to_string(), group: "core".to_string() });

    for k in karmas {
        let kid = format!("karma-{}", k.get("id").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("unknown"));
        let lesson = k.get("lesson").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("Lesson");
        let node_id = k.get("node_id").and_then(|v| v.as_str()).unwrap_or("");
        
        let group = if node_id == local_node_id || node_id.is_empty() {
            "karma_local"
        } else {
            "karma_global"
        };

        nodes.push(GraphNode {
            id: kid.clone(),
            label: lesson.chars().take(30).collect::<String>() + "...",
            group: group.to_string(),
        });

        edges.push(GraphEdge { from: "aiome-core".to_string(), to: kid });
    }

    for rule in rules {
        let rid = format!("rule-{}", rule.id);
        nodes.push(GraphNode {
            id: rid.clone(),
            label: format!("[RULE] {}", rule.pattern),
            group: "immune".to_string(),
        });
        edges.push(GraphEdge { from: "aiome-core".to_string(), to: rid });
    }

    Json(GraphData { nodes, edges })
}

pub async fn get_immune_rules_handler(
    State(state): State<AppState>,
) -> Json<Vec<aiome_core::contracts::ImmuneRule>> {
    let rules = state.job_queue.get_immune_rules().await.unwrap_or_default();
    Json(rules)
}

pub async fn add_immune_rule_handler(
    State(state): State<AppState>,
    Json(mut rule): Json<aiome_core::contracts::ImmuneRule>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // Phase 20 MVP: Generate ID and timestamp if empty
    if rule.id.is_empty() {
        rule.id = uuid::Uuid::new_v4().to_string();
    }
    if rule.created_at.is_empty() {
        rule.created_at = chrono::Utc::now().to_rfc3339();
    }

    state.job_queue.store_immune_rule(&rule).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;
    
    Ok(Json(serde_json::json!({"status": "success", "id": rule.id})))
}

pub async fn delete_immune_rule_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.job_queue.delete_immune_rule(&id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))))?;
    
    Ok(Json(serde_json::json!({"status": "success"})))
}

pub async fn get_evolution_history_handler(
    State(state): State<AppState>,
) -> Json<Vec<serde_json::Value>> {
    let history = state.job_queue.fetch_evolution_history(50).await.unwrap_or_default();
    Json(history)
}
