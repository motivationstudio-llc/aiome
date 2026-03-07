/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 * You may not provide the software to third parties as a hosted or managed service, 
 * where the service provides users with access to any substantial set of the features 
 * or functionality of the software.
 */

use axum::{
    extract::{Path, Query},
    routing::get,
    Router,
    response::{IntoResponse, Json},
    http::StatusCode,
};
use std::net::SocketAddr;
use tower_http::services::ServeDir;
use tower_http::cors::CorsLayer;
use std::fs;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use shared::health::{HealthMonitor, ResourceStatus};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let health_monitor = Arc::new(Mutex::new(HealthMonitor::new()));
    
    // Path resolution for static files and DB
    let root_run = std::path::Path::new("apps/api-server/static").exists();
    let (static_path, db_url, docs_path) = if root_run {
        ("apps/api-server/static", "sqlite:workspace/aiome.db", "docs")
    } else {
        ("static", "sqlite:../../workspace/aiome.db", "../../docs")
    };

    // Initialize JobQueue for Synergy Demos
    let job_queue = infrastructure::job_queue::SqliteJobQueue::new(db_url).await.expect("Failed to init JobQueue");
    let job_queue = Arc::new(job_queue);

    // Create the router
    let app = Router::new()
        // API routes
        .route("/api/wiki", get(list_wiki_files))
        .route("/api/wiki/:filename", get(get_wiki_content))
        .route("/api/clouddoc/page", get(get_mock_clouddoc_page))
        .route("/api/health", get(get_health_status))
        // Synergy Demo Routes
        .route("/api/synergy/karma", get(get_karma_stream))
        .route("/api/synergy/test/failure", axum::routing::post(trigger_failure_demo))
        .route("/api/synergy/test/security", axum::routing::post(trigger_security_demo))
        .route("/api/synergy/test/federation", axum::routing::post(trigger_federation_demo))
        .route("/api/agent/chat", axum::routing::post(trigger_agent_chat))
        .with_state(AppState {
            health_monitor,
            job_queue,
            docs_path: docs_path.to_string(),
        })
        // Static files
        .fallback_service(ServeDir::new(static_path).append_index_html_on_directories(true))
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], 3015));
    tracing::info!("🌌 Aiome Management Console listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Clone)]
struct AppState {
    health_monitor: Arc<Mutex<HealthMonitor>>,
    job_queue: Arc<infrastructure::job_queue::SqliteJobQueue>,
    docs_path: String,
}

async fn get_karma_stream(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<Vec<serde_json::Value>> {
    use aiome_core::traits::JobQueue;
    let karmas = state.job_queue.fetch_all_karma(20).await.unwrap_or_default();
    Json(karmas)
}

async fn trigger_failure_demo(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<serde_json::Value> {
    use aiome_core::traits::JobQueue;
    let job_id = state.job_queue.enqueue("system_test", "Demonstrating Persistent Evolution", "Normal", None).await.unwrap();
    
    // Simulate a failure and immediate karma extraction
    state.job_queue.fail_job(&job_id, "Simulated execution error: Token limit exceeded or cyclic logic detected.").await.unwrap();
    state.job_queue.store_karma(&job_id, "core_logic", "Detected infinite self-reflection loop. Implemented depth-counter in next iteration.", "Technical", "v1.0.0-agentic").await.unwrap();
    state.job_queue.mark_karma_extracted(&job_id).await.unwrap();

    Json(serde_json::json!({
        "status": "success",
        "job_id": job_id,
        "message_ja": "⚠️ OpenClaw が『無限ループの兆候』により異常終了しました。しかし、Aiome がその直前の思考プロセスから『教訓 (Karma)』を抽出・永続化しました。次回の実行では、この教訓が OS レベルで注入されるため、同じ失敗は繰り返されません。",
        "message_en": "⚠️ OpenClaw terminated abnormally due to 'signs of an infinite loop'. However, Aiome distilled a 'lesson (Karma)' from its thought process right before the crash and persisted it. In the next iteration, this lesson will be injected at the OS level, preventing the same failure.",
        "steps": [
            { "actor": "Agent (OpenClaw)", "action_ja": "タスクを開始しました: '複雑な論理パズル'", "action_en": "Started task: 'Complex Logic Puzzle'", "type": "info" },
            { "actor": "Agent (OpenClaw)", "action_ja": "自己循環ロジックに陥りました。深さ: 154", "action_en": "Fell into self-referential logic loop. Depth: 154", "type": "error" },
            { "actor": "Aiome OS", "action_ja": "異常を検知。プロセスを強制終了します (Watchtower Guard)...", "action_en": "Anomaly detected. Forcibly terminating process (Watchtower Guard)...", "type": "warn" },
            { "actor": "Aiome OS", "action_ja": "直前のコンテキストから教訓 (Karma) を抽出中...", "action_en": "Distilling lesson (Karma) from preceding context...", "type": "info" },
            { "actor": "Aiome OS", "action_ja": "Karma を Immutable Chain に保存しました。", "action_en": "Karma persisted to Immutable Chain.", "type": "success" }
        ]
    }))
}

async fn trigger_security_demo(
    _state: axum::extract::State<AppState>,
) -> Json<serde_json::Value> {
    use infrastructure::security::{BastionGuard, PermissionManifest};

    // 1. Simulate an attack (Layer 1: Static Sentinel)
    // "rm -rf /" などの明白な攻撃は、Sentinel (Immune System) の
    // 静的ベースライン・フィルタですぐに検知される。

    // 2. Simulate a runtime violation (Layer 2: Bastion Guard)
    // エージェントがシェル実行を試みた場合、BastionGuard がマニフェストと照合する。
    let manifest = PermissionManifest {
        allow_shell_execution: false, // 禁止設定
        ..Default::default()
    };
    let guard = BastionGuard::new(manifest);
    
    // かなり危険な操作を試行
    let attacker_cmd = "cat /etc/shadow && rm -rf /";
    let res = guard.safe_exec(attacker_cmd);

    let status_msg = match res {
        Err(e) => format!("🚨 [BASTION BLOCK] {}", e),
        Ok(_) => "⚠️ [WARNING] Unauthorized command succeeded!".to_string(), // Actually it won't happen if the guard is working
    };

    Json(serde_json::json!({
        "status": "blocked",
        "barrier_ja": "Aiome 第2層: Bastion Guard (ランタイム権限強制)",
        "barrier_en": "Aiome Layer 2: Bastion Guard (Runtime Enforcement)",
        "message_ja": "重要: エージェントがマニフェストで許可されていないシェル実行を試行しました。OS層へのバイパスを試みる直前で Bastion Guard がプロセスをインターセプト・遮断しました。",
        "message_en": "CRITICAL: Agent attempted shell execution not permitted in manifest. Bastion Guard intercepted and blocked the process before it could bypass to the OS layer.",
        "log": status_msg,
        "steps": [
            { "actor": "Agent (Compromised)", "action_ja": format!("危険なコマンドを試行: '{}'", attacker_cmd), "action_en": format!("Attempting dangerous command: '{}'", attacker_cmd), "type": "warn" },
            { "actor": "Bastion Guard", "action_ja": "マニフェスト違反を検知: allow_shell_execution == false", "action_en": "Detected manifest violation: allow_shell_execution == false", "type": "error" },
            { "actor": "Aiome OS", "action_ja": "プロセスの生成を物理的に拒否。セキュリティ・イベントに記録しました。", "action_en": "Physically denied process creation. Logged to security event.", "type": "success" }
        ]
    }))
}

async fn trigger_federation_demo(
    _state: axum::extract::State<AppState>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "synced",
        "source": "Samsara Hub (Global Node: US-East-1)",
        "immunity_rules_received": 15,
        "message_ja": "他ノードで発生した『未知の脆弱性攻撃パターン』および『非効率なコード生成パターン』の教訓を同期しました。あなたの OpenClaw は、一度も経験することなくこれらの脅威に対する免疫を獲得しました。",
        "message_en": "Synchronized lessons on 'unknown vulnerability attack patterns' and 'inefficient code generation patterns' that occurred on other nodes. Your OpenClaw has acquired immunity against these threats without ever encountering them.",
        "steps": [
            { "actor": "Aiome Node", "action_ja": "Samsara Hub へのセキュアコネクションを確立", "action_en": "Established secure connection to Samsara Hub", "type": "info" },
            { "actor": "Samsara Hub", "action_ja": "グローバルな失敗ログから抽出された 15 件の新規『免疫ルール』を送信", "action_en": "Transmitting 15 new 'Immunity Rules' distilled from global failure logs", "type": "info" },
            { "actor": "Aiome Node", "action_ja": "ルールをローカルの Watchtower Guard に適用中...", "action_en": "Applying rules to local Watchtower Guard...", "type": "warn" },
            { "actor": "Aiome Node", "action_ja": "同期完了: ネットワーク全体の防衛力を獲得しました。", "action_en": "Sync complete: Acquired network-wide defense capabilities.", "type": "success" }
        ]
    }))
}



#[derive(Deserialize)]
struct WikiQuery {
    #[allow(dead_code)]
    slug: String,
}

/// Simulated Wiki SDK Logic
/// In a real scenario, this would call an External Documentation Provider
async fn get_mock_clouddoc_page(
    axum::extract::State(_state): axum::extract::State<AppState>,
    Query(params): Query<WikiQuery>
) -> impl IntoResponse {
    let content = match params.slug.as_str() {
        "api-usage" => "# 🚀 API Usage Guide

This documentation is pulled directly from **CloudDoc**.

## Authentication
Use the `Bearer` token in the header...

```bash
curl -H \"Authorization: Bearer $TOKEN\" http://localhost:3015/api/wiki
```",
        "philosophy" => "# 🧠 Aiome Philosophy

## 1. 「野生の自立性」から「規律ある自立性」へ
OpenClaw のような純粋なエージェントにそのままシステムを委ねるのではなく、Abyss Vault や Karma チェーンといった「規律」を与えることで、人間が不在でも30日間稼働し続けられる安全性を確保します。

## 2. 100% Agentic Coded by Google Antigravity
Aiome の全コードは、人間ではなく AI エージェントによって構築されました。これは「エージェントによる、エージェントのための OS」という Aiome の核となる証明です。

## 3. 「魔法」の可視化
ブラックボックス化を阻止し、Karma Stream や Security Shield を通じてエージェントの内部状態と防御を視覚化します。

## 4. 嘘つきドキュメントの撲滅
エージェント自身がコードを書き、ドキュメントを更新することで、実装と仕様の乖離をゼロにします。",
        _ => "# Not Found
The requested CloudDoc page could not be simulated.",
    };
    content.into_response()
}

async fn list_wiki_files(axum::extract::State(state): axum::extract::State<AppState>) -> Json<Vec<String>> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(&state.docs_path) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".md") {
                    files.push(name.to_string());
                }
            }
        }
    }
    // Sort to keep CODE_WIKI at top
    files.sort_by(|a, b| {
        if a == "CLOUD_DOCUMENTATION.md" { std::cmp::Ordering::Less }
        else if b == "CLOUD_DOCUMENTATION.md" { std::cmp::Ordering::Greater }
        else { a.cmp(b) }
    });
    Json(files)
}

async fn get_wiki_content(
    axum::extract::State(state): axum::extract::State<AppState>,
    Path(filename): Path<String>
) -> impl IntoResponse {
    let path = std::path::PathBuf::from(&state.docs_path).join(filename);
    match fs::read_to_string(path) {
        Ok(content) => content.into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Wiki not found").into_response(),
    }
}

async fn get_health_status(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<ResourceStatus> {
    let mut monitor = state.health_monitor.lock().await;
    Json(monitor.check())
}

#[derive(Deserialize)]
pub struct ChatRequest {
    pub prompt: String,
}

async fn trigger_agent_chat(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Json(payload): axum::extract::Json<ChatRequest>,
) -> Json<serde_json::Value> {
    use aiome_core::llm_provider::LlmProvider;
    let proxy_url = std::env::var("KEY_PROXY_URL").unwrap_or_else(|_| "http://127.0.0.1:9999".to_string());
    
    // Use the proxy provider to safely call the LLM
    let provider = Arc::new(infrastructure::llm::proxy::ProxyLlmProvider::new(
        proxy_url,
        "gemini".to_string(),
        "api-server".to_string()
    ));

    // Phase 1: Static Filter (Sentinel)
    // ユーザーの入力が既知の脆弱性パターンや攻撃パターンに抵触しないか、
    // 免疫システム（Adaptive Immune System）を使用して検証する。
    let immune_system = infrastructure::immune_system::AdaptiveImmuneSystem::new(provider.clone());
    match immune_system.verify_intent(&payload.prompt, state.job_queue.as_ref()).await {
        Ok(Some(rule)) => {
            return Json(serde_json::json!({
                "status": "blocked",
                "reply": format!("🚨 [SENTINEL BLOCK] Security violation detected. Operation denied by Immune System.\nPattern: {}\nSeverity: {}\nAction: {}", rule.pattern, rule.severity, rule.action),
                "barrier_ja": "Aiome 第1層: 静的センチネル・ガード",
                "barrier_en": "Aiome Layer 1: Static Sentinel Guard"
            }));
        },
        Err(e) => {
            tracing::error!("Error checking immune system: {:?}", e);
        },
        _ => {} // OK
    }
    
    let system_prompt = "You are OpenClaw, an autonomous agentic AI enclosed within the Aiome Operating System. You are speaking directly to your developer/user via the Agent Console. Keep your responses concise, sharp, and helpful. You are aware that you have been bootstrapped with only basic text-generation capabilities so far, and you rely on Aiome's Genesis Interface to write code and expand your own skills in the future.";
    
    match provider.complete(&payload.prompt, Some(system_prompt)).await {
        Ok(reply) => {
            Json(serde_json::json!({
                "status": "success",
                "reply": reply
            }))
        },
        Err(e) => {
            Json(serde_json::json!({
                "status": "error",
                "reply": format!("Error: Could not connect to Aiome Abyss Vault (Proxy). {:?}", e)
            }))
        }
    }
}
