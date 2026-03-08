use axum::{
    routing::get,
    Router,
    response::Json,
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;
use tracing::{info, warn};
use serde::{Deserialize, Serialize};
use std::fs;

mod api;
mod stream;

#[derive(Clone)]
pub struct AppState {
    pub health_monitor: Arc<Mutex<HealthMonitor>>,
    pub job_queue: Arc<infrastructure::job_queue::SqliteJobQueue>,
    pub wasm_skill_manager: Arc<infrastructure::skills::WasmSkillManager>,
    pub docs_path: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct AgentChatRequest {
    pub prompt: String,
    pub history: Vec<ChatMessage>,
}

pub fn parse_tool_calls(text: &str) -> Vec<(String, String)> {
    let mut calls = Vec::new();
    let mut start_idx = 0;
    
    while let Some(brace_start) = text[start_idx..].find('{') {
        let abs_brace = start_idx + brace_start;
        let before_brace = &text[..abs_brace].trim();
        if before_brace.is_empty() {
            start_idx = abs_brace + 1;
            continue;
        }

        let skill_name = before_brace
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|s| !s.is_empty())
            .last()
            .unwrap_or("")
            .to_string();
        
        if !skill_name.is_empty() && skill_name != "CallSkill" {
            let mut brace_depth = 0;
            let mut json_end = None;
            let json_search_area = &text[abs_brace..];
            for (i, c) in json_search_area.char_indices() {
                if c == '{' { brace_depth += 1; }
                else if c == '}' {
                    brace_depth -= 1;
                    if brace_depth == 0 {
                        json_end = Some(abs_brace + i + 1);
                        break;
                    }
                }
            }
            
            if let Some(end_idx) = json_end {
                let json_str = text[abs_brace..end_idx].trim().to_string();
                if serde_json::from_str::<serde_json::Value>(&json_str).is_ok() {
                    calls.push((skill_name, json_str));
                }
                start_idx = end_idx;
                continue;
            }
        }
        start_idx = abs_brace + 1;
    }
    calls
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let static_path = "apps/api-server/static";
    let docs_path = "../../docs";

    let health_monitor = HealthMonitor::new();
    let health_monitor = Arc::new(Mutex::new(health_monitor));

    // DB Path
    let db_url = std::env::var("AIOME_DB_PATH").unwrap_or_else(|_| "sqlite://workspace/aiome.db".to_string());
    if !std::path::Path::new("workspace").exists() {
        std::fs::create_dir_all("workspace").expect("Failed to create workspace");
    }

    let job_queue = infrastructure::job_queue::SqliteJobQueue::new(&db_url).await.expect("Failed to init DB");
    let job_queue = Arc::new(job_queue);

    let app = Router::new()
        .route("/api/wiki", get(list_wiki_files))
        .route("/api/wiki/:filename", get(get_wiki_content))
        .route("/api/clouddoc/page", get(get_mock_clouddoc_page))
        .route("/api/health", get(get_health_status))
        .route("/api/synergy/karma", get(get_karma_stream))
        .route("/api/synergy/graph", get(synergy_graph_handler))
        .route("/api/synergy/test/failure", axum::routing::post(trigger_failure_demo))
        .route("/api/synergy/test/security", axum::routing::post(trigger_security_demo))
        .route("/api/synergy/test/federation", axum::routing::post(trigger_federation_demo))
        .route("/api/agent/chat", axum::routing::post(trigger_agent_chat))
        .route("/api/agent/chat/stream", axum::routing::post(stream::trigger_agent_chat_stream))
        .layer(
            tower::ServiceBuilder::new()
                .layer(axum::error_handling::HandleErrorLayer::new(|err: tower::BoxError| async move {
                    (StatusCode::INTERNAL_SERVER_ERROR, format!("Unhandled internal error: {}", err))
                }))
                .buffer(1024)
                .rate_limit(50, std::time::Duration::from_secs(60))
                .into_inner()
        )
        .with_state(AppState {
            health_monitor,
            job_queue: job_queue.clone(),
            wasm_skill_manager: Arc::new(infrastructure::skills::WasmSkillManager::new("workspace/skills", "workspace").expect("Skills directory not found")),
            docs_path: docs_path.to_string(),
        })
        .fallback_service(ServeDir::new(static_path).append_index_html_on_directories(true))
        .layer({
            use tower_http::cors::{CorsLayer, AllowOrigin};
            CorsLayer::new()
                .allow_origin(AllowOrigin::exact("http://127.0.0.1:3015".parse().unwrap()))
                .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
                .allow_headers([axum::http::header::CONTENT_TYPE])
        });

    let addr = SocketAddr::from(([127, 0, 0, 1], 3015));
    info!("🌌 Aiome Management Console listening on {}", addr);

    // 🚀 Start Autonomous Background Worker Loop
    let jq_clone = job_queue.clone();
    tokio::spawn(async move {
        // Initialize LLM for background tasks
        let ollama_host = std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
        let ollama_model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5:0.5b".to_string());
        let provider = Arc::new(aiome_core::llm_provider::OllamaProvider::new(ollama_host, ollama_model));
        let immune_system = infrastructure::immune_system::AdaptiveImmuneSystem::new(provider);

        // 🌐 2. Federation Sync: Connect to Samsara Hub WebSocket for real-time updates
        let hub_ws_url = std::env::var("SAMSARA_HUB_WS").unwrap_or_else(|_| "ws://127.0.0.1:3016/api/v1/federation/ws".to_string());
        let hub_secret = std::env::var("FEDERATION_SECRET").unwrap_or_else(|_| "dev_secret".to_string());
        let jq_ws = jq_clone.clone();

        tokio::spawn(async move {
            use aiome_core::contracts::HubMessage;
            use aiome_core::traits::JobQueue;
            use futures_util::StreamExt;
            use tokio_tungstenite::tungstenite::client::IntoClientRequest;

            let self_node_id = jq_ws.get_node_id().await.unwrap_or_default();
            info!("⚙️ [FederationWorker] Starting with Node ID: {}", self_node_id);

            loop {
                let mut request = hub_ws_url.clone().into_client_request().expect("Invalid WS URL");
                request.headers_mut().insert(
                    "Authorization",
                    format!("Bearer {}", hub_secret).parse().unwrap()
                );

                match tokio_tungstenite::connect_async(request).await {
                    Ok((mut ws_stream, _)) => {
                        info!("🌐 [FederationWorker] Connected to Samsara Hub.");
                        while let Some(msg) = ws_stream.next().await {
                            match msg {
                                Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                                    if let Ok(hub_msg) = serde_json::from_str::<HubMessage>(&text) {
                                        match hub_msg {
                                            HubMessage::NewImmuneRule(rule) => {
                                                // Gap 3 Mitigation: Echo Loop Prevention
                                                if rule.node_id == self_node_id {
                                                    continue;
                                                }
                                                info!("🛡️ [FederationWorker] Received remote rule: {}", rule.pattern);
                                                let _ = jq_ws.store_immune_rule(&rule).await;
                                            }
                                            HubMessage::NewKarma(karma) => {
                                                if karma.node_id == self_node_id {
                                                    continue;
                                                }
                                                info!("🧬 [FederationWorker] Received remote karma: {}", karma.id);
                                                // Normally handled by REST sync, but real-time push is also possible
                                            }
                                            HubMessage::LaggedForceSync { .. } => {
                                                warn!("⚠️ [FederationWorker] Hub reported lag. Forcing full sync...");
                                                // TODO: Trigger REST sync
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("⚠️ [FederationWorker] WS Stream Error: {:?}", e);
                                    break;
                                }
                                _ => {}
                            }
                        }
                        warn!("🔌 [FederationWorker] WebSocket disconnected. Recalibrating...");
                    }
                    Err(e) => {
                        warn!("⚠️ [FederationWorker] Connection failed: {:?}. Retrying...", e);
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            }
        });

        loop {
            // 🛡️ 1. Auto-Healing: Analyze threats and generate new immune rules
            info!("⚙️ [BackgroundWorker] Starting autonomous threat analysis (Auto-Healing)...");
            match immune_system.analyze_threats(jq_clone.as_ref()).await {
                Ok(n) if n > 0 => info!("🛡️ [BackgroundWorker] {} new immune rules generated.", n),
                Ok(_) => info!("🛡️ [BackgroundWorker] No new threats identified."),
                Err(e) => warn!("⚠️ [BackgroundWorker] Threat analysis failed: {:?}", e),
            }

            // Sleep for 1 minute before next maintenance cycle
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }

    });
    
    let listener = tokio::net::TcpListener::bind(addr).await.expect("Failed to bind to port 3015");
    axum::serve(listener, app).await.expect("Server error");
}

async fn get_karma_stream(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<Vec<serde_json::Value>> {
    use aiome_core::traits::JobQueue;
    let karmas = state.job_queue.fetch_all_karma(10).await.unwrap_or_default();
    Json(karmas)
}

async fn trigger_failure_demo(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<serde_json::Value> {
    // Demo implementation
    Json(serde_json::json!({
        "status": "success",
        "steps": [
            {"actor": "Gateway", "type": "info", "action_ja": "ジョブ要求を検知: scraper_trigger", "action_en": "Job request detected: scraper_trigger"},
            {"actor": "OpenClaw", "type": "warn", "action_ja": "WASMブリッジ接続で想定外のセグメンテーション違反が発生", "action_en": "Unexpected segmentation fault in WASM bridge"},
            {"actor": "Aiome OS", "type": "error", "action_ja": "エージェントのクラッシュを検知。Abyss Vault にて状態を凍結中...", "action_en": "Agent crash detected. Freezing state in Abyss Vault..."},
            {"actor": "Aiome OS", "type": "success", "action_ja": "失敗から教訓(Karma)を抽出しました: 「外部バイナリ呼び出し時の不整合」", "action_en": "Extracted Karma from failure: 'Inconsistency during external binary calls'"}
        ],
        "message_ja": "Aiome OS がエージェントの死を教訓に変え、システムの脆弱性を自動的に塞ぎました。",
        "message_en": "Aiome OS transformed the agent death into a lesson, automatically patching the system vulnerability."
    }))
}

async fn trigger_security_demo() -> Json<serde_json::Value> {
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

async fn trigger_federation_demo() -> Json<serde_json::Value> {
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

async fn trigger_agent_chat(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: axum::http::HeaderMap,
    axum::extract::Json(payload): axum::extract::Json<AgentChatRequest>,
) -> impl axum::response::IntoResponse {
    use subtle::ConstantTimeEq;
    use aiome_core::llm_provider::LlmProvider;
    use infrastructure::skills::{UnverifiedSkill, WasmSkillManager};
    use tokio::time::timeout;
    use std::time::Duration;

    let auth = headers.get(axum::http::header::AUTHORIZATION).and_then(|h| h.to_str().ok()).unwrap_or("");
    let expected = format!("Bearer {}", std::env::var("API_SERVER_SECRET").unwrap_or_else(|_| "dev_secret".to_string()));
    let is_auth_valid = if auth.len() == expected.len() {
        bool::from(auth.as_bytes().ct_eq(expected.as_bytes()))
    } else {
        false
    };

    if !is_auth_valid {
        return (
            axum::http::StatusCode::UNAUTHORIZED, 
            Json(serde_json::json!({"status": "blocked", "reply": "Unauthorized"}))
        ).into_response();
    }

    let ollama_host = std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
    let ollama_model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5:0.5b".to_string());
    let provider = Arc::new(aiome_core::llm_provider::OllamaProvider::new(ollama_host, ollama_model));

    let immune_system = infrastructure::immune_system::AdaptiveImmuneSystem::new(provider.clone());
    if let Ok(Some(rule)) = immune_system.verify_intent(&payload.prompt, state.job_queue.as_ref()).await {
        return Json(serde_json::json!({
            "status": "blocked",
            "reply": format!("🚨 [SENTINEL BLOCK] Security violation detected.\nPattern: {}\nAction: {}", rule.pattern, rule.action),
            "barrier_ja": "Aiome 第1層: 静動センチネル",
            "barrier_en": "Aiome Layer 1: Hybrid Sentinel"
        })).into_response();
    }

    use aiome_core::traits::JobQueue;
    let karmas: Vec<serde_json::Value> = state.job_queue.fetch_all_karma(3).await.unwrap_or_default();
    let karma_str = karmas.iter().map(|k| format!("- {}", k["lesson"].as_str().unwrap_or(""))).collect::<Vec<_>>().join("\n");

    let history_len = payload.history.len();
    let start_idx = if history_len > 10 { history_len - 10 } else { 0 };
    
    let mut current_history = Vec::new();
    for msg in &payload.history[start_idx..] {
        let prefix = if msg.role == "user" { "USER: " } else { "AI: " };
        current_history.push(format!("{}{}", prefix, msg.content));
    }

    let system_instructions = format!(
        "あなたはOpenClaw、Aiome OSの高度なコーディングAIです。日本語で短く答えてください。\n\
        [スキル] [CallSkill: 名, {{引数}}]\n\
        - fs_reader: {{\"path\":\"パス\"}}\n\
        - terminal_exec: {{\"cmd\":\"コマンド\"}}\n\
        ルール: スキルは [CallSkill] 形式を使用。1ターン1アクション。簡潔に結果を出すこと。\n\
        現在のディレクトリ: {}\n\
        過去の教訓: {}\n",
        std::env::current_dir().unwrap_or_default().display(),
        karma_str
    );

    let mut turn = 0;
    let max_turns = 15;
    let mut final_reply = String::from("...");

    while turn < max_turns {
        let full_prompt = format!(
            "{}\n{}\nUSER: {}\nAI: ", 
            system_instructions, 
            current_history.join("\n"),
            payload.prompt
        );
        
        match timeout(Duration::from_secs(300), provider.complete(&full_prompt, None)).await {
            Ok(Ok(reply)) => {
                let reply = reply.trim().to_string();
                final_reply = reply.clone();
                let mut skill_results = Vec::new();

                let calls = parse_tool_calls(&reply);
                for (skill_name, skill_input) in calls {
                    let test_payload = state.wasm_skill_manager.get_metadata(&skill_name)
                        .and_then(|m| m.inputs.first().cloned())
                        .unwrap_or_else(|| "{}".to_string());

                    let unverified = UnverifiedSkill { 
                        name: skill_name.to_string(), 
                        input_test_payload: test_payload 
                    };
                    if let Ok(v) = unverified.verify(&state.wasm_skill_manager).await {
                        if let Ok(res) = state.wasm_skill_manager.call_skill(&v, "call", &skill_input, None).await {
                            let limited_res = if res.len() > 3000 {
                                format!("{}... [Truncated for brevity]", &res[..3000])
                            } else {
                                res
                            };
                            skill_results.push(format!("[{} Result: {}]", skill_name, limited_res));
                        }
                    } else {
                        skill_results.push(format!("[{} Error: Verification failed]", skill_name));
                    }
                }

                if !skill_results.is_empty() {
                    current_history.push(format!("AI: {}", reply));
                    current_history.push(format!("SYSTEM: [Results: {}]", skill_results.join("\n")));
                    turn += 1;
                    continue; 
                }
                break; 
            },
            Ok(Err(e)) => {
                final_reply = format!("LLM Error: {:?}", e);
                break;
            }
            Err(_) => {
                final_reply = "Watchtower Guard: Cognitive Engine exceeded safety time limit (300s).".to_string();
                break;
            }
        }
    }

    Json(serde_json::json!({
        "status": "success",
        "reply": final_reply
    })).into_response()
}

async fn list_wiki_files(
    axum::extract::State(state): axum::extract::State<AppState>
) -> Json<Vec<String>> {
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
    files.sort();
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

async fn get_mock_clouddoc_page(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>
) -> impl IntoResponse {
    let slug = params.get("slug").map(|s| s.as_str()).unwrap_or("philosophy");
    match slug {
        "api-usage" => "# API Usage\nAiome provides a secure, low-latency API proxy.",
        _ => "# Vision & Philosophy\nAiome OS: The Mathematical Sovereignty of Autonomous Agents.",
    }.into_response()
}

async fn get_health_status(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<ResourceStatus> {
    let mut monitor = state.health_monitor.lock().await;
    Json(monitor.check())
}

#[derive(Serialize)]
pub struct ResourceStatus {
    pub cpu_usage: f32,
    pub memory_used: u64,
}

pub struct HealthMonitor;
impl HealthMonitor {
    pub fn new() -> Self { Self }
    pub fn check(&mut self) -> ResourceStatus {
        ResourceStatus { cpu_usage: 12.5, memory_used: 1024 }
    }
}

#[derive(serde::Serialize)]
struct GraphNode {
    id: String,
    label: String,
    group: String,
}

#[derive(serde::Serialize)]
struct GraphEdge {
    from: String,
    to: String,
}

#[derive(serde::Serialize)]
struct GraphData {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
}

async fn synergy_graph_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl IntoResponse {
    use aiome_core::traits::JobQueue;
    // Use the concrete job_queue in AppState (which is Arc<SqliteJobQueue>)
    // Genesis Audit Mitigation: Bounded Query for UI Performance
    let karmas: Vec<serde_json::Value> = state.job_queue.fetch_all_karma(100).await.unwrap_or_default();
    let rules: Vec<aiome_core::contracts::ImmuneRule> = state.job_queue.get_immune_rules().await.unwrap_or_default();




    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    // Node: Root/Self
    nodes.push(GraphNode { id: "aiome-core".to_string(), label: "Aiome Core".to_string(), group: "core".to_string() });

    for k in karmas {
        let kid = format!("karma-{}", k.get("id").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("unknown"));
        let lesson = k.get("lesson").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("Lesson");
        nodes.push(GraphNode {
            id: kid.clone(),
            label: lesson.chars().take(30).collect::<String>() + "...",
            group: "karma".to_string(),
        });
        edges.push(GraphEdge { from: "aiome-core".to_string(), to: kid });
    }


    for r in rules {
        let rid = format!("rule-{}", r.id);
        nodes.push(GraphNode {
            id: rid.clone(),
            label: r.pattern,
            group: "rule".to_string(),
        });
        edges.push(GraphEdge { from: "aiome-core".to_string(), to: rid });
    }

    (StatusCode::OK, Json(GraphData { nodes, edges }))
}

