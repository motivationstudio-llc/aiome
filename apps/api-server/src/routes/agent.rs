use axum::{
    response::Json,
    response::IntoResponse,
    extract::State,
    http::HeaderMap,
    http::StatusCode,
};
use tracing::info;
use std::sync::Arc;
use tokio::time::timeout;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use crate::AppState;
use crate::skill_handler;
use crate::docker;
use aiome_core::llm_provider::LlmProvider;
use aiome_core::traits::JobQueue;

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

pub fn build_system_instructions(state: &AppState, karma_str: &str) -> String {
    let skill_list = state.wasm_skill_manager.list_skills_with_metadata()
        .iter()
        .map(|m| {
            let desc = if m.description == "No metadata provided" { "" } else { &m.description };
            format!("- {}: {} (入力: {})", m.name, desc, m.inputs.first().cloned().unwrap_or_default())
        })
        .collect::<Vec<_>>()
        .join("\n");
        
    let soul = std::fs::read_to_string("SOUL.md").unwrap_or_default();
    let evolving_soul = std::fs::read_to_string("EVOLVING_SOUL.md").unwrap_or_default();
    let forge_prompt = std::fs::read_to_string("workspace/config/SKILL_FORGE_PROMPT.md").unwrap_or_default();
    
    let identity_prefix = if !soul.is_empty() || !evolving_soul.is_empty() {
        format!("# IDENTITY: あなたはAiomeの守護者(Watchtower)です。🐾\n\
                ルール: 簡潔に答え、[CallSkill]以外は自然な文章で話してください。私的な情報は守秘してください。\n\n")
    } else {
        "あなたはOpenClaw、Aiome OSの高度なコーディングAIです。日本語で短く答えてください。\n\n".to_string()
    };
    
    format!(
        "{}[利用可能な Wasm スキル]\n\
        {}\n\n\
        [Forge ツール (内部コマンド)]\n\
        - forge_skill: {{\"skill_name\": \"...\", \"initial_rust_code\": \"...\", \"description\": \"...\"}}\n\
        - forge_test_run: {{\"skill_name\": \"...\", \"test_input\": \"...\"}}\n\
        - forge_publish: {{\"skill_name\": \"...\"}}\n\n\
        ルール:\n\
        1. スキル・ツールは [CallSkill: 名, {{引数}}] 形式を使用。\n\
        2. 1ターンにつき1つの [CallSkill] のみ実行可能。\n\
        3. 自分が現在使えるスキルの全スキーマは上記リストを参照。\n\n\
        現在のディレクトリ: {}\n\
        過去の教訓: {}\n\n\
        {}\n",
        identity_prefix,
        skill_list,
        std::env::current_dir().unwrap_or_default().display(),
        karma_str,
        forge_prompt
    )
}

pub async fn trigger_agent_chat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<AgentChatRequest>,
) -> impl IntoResponse {
    use subtle::ConstantTimeEq;

    let auth = headers.get(axum::http::header::AUTHORIZATION).and_then(|h| h.to_str().ok()).unwrap_or("");
    let expected = format!("Bearer {}", std::env::var("API_SERVER_SECRET").unwrap_or_else(|_| "dev_secret".to_string()));
    let is_auth_valid = if auth.len() == expected.len() {
        bool::from(auth.as_bytes().ct_eq(expected.as_bytes()))
    } else {
        false
    };

    if !is_auth_valid {
        return (
            StatusCode::UNAUTHORIZED, 
            Json(serde_json::json!({"status": "blocked", "reply": "Unauthorized"}))
        ).into_response();
    }

    if let shared::guardrails::ValidationResult::Blocked(reason) = shared::guardrails::validate_input(&payload.prompt) {
        return Json(serde_json::json!({
            "status": "blocked",
            "reply": format!("🚨 [GUARDRAIL BLOCK] {}", reason)
        })).into_response();
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

    let karmas: Vec<serde_json::Value> = state.job_queue.fetch_all_karma(3).await.unwrap_or_default();
    let karma_str = karmas.iter().map(|k| format!("- {}", k["lesson"].as_str().unwrap_or(""))).collect::<Vec<_>>().join("\n");

    let history_len = payload.history.len();
    let start_idx = if history_len > 10 { history_len - 10 } else { 0 };
    
    let mut current_history = Vec::new();
    for msg in &payload.history[start_idx..] {
        let prefix = if msg.role == "user" { "USER: " } else { "AI: " };
        current_history.push(format!("{}{}", prefix, msg.content));
    }

    let system_instructions = build_system_instructions(&state, &karma_str);

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
        
        let _llm_permit = state.llm_semaphore.acquire().await.ok();

        match timeout(Duration::from_secs(300), provider.complete(&full_prompt, None)).await {
            Ok(Ok(reply)) => {
                let reply = reply.trim().to_string();
                final_reply = reply.clone();
                let mut skill_results = Vec::new();

                let calls = parse_tool_calls(&reply);
                for (skill_name, skill_input) in calls {
                    info!("🛠️ [AgentLoop] Executing skill: {}", skill_name);
                    
                    if skill_name.starts_with("forge_") {
                        match skill_handler::execute_forge_command(&skill_name, &skill_input, &state).await {
                            Ok(res) => skill_results.push(res),
                            Err(e) => skill_results.push(format!("[{} Error: {}]", skill_name, e)),
                        }
                    } else {
                        let res = skill_handler::execute_wasm_skill(&skill_name, &skill_input, &state).await;
                        skill_results.push(res);
                    }
                }

                if reply.contains("[DelegateDocker") {
                    if let Some(brace_start) = reply.find("[DelegateDocker") {
                        let content = &reply[brace_start + 15 ..];
                        if let Some(brace_end) = content.find(']') {
                            let json_str = &content[..brace_end];
                            #[derive(serde::Deserialize)]
                            struct DockerReq { agent_yaml: String, task: String }
                            if let Ok(req) = serde_json::from_str::<DockerReq>(json_str) {
                                info!("🐳 [AgentLoop] Delegating task to Docker Shadow Worker...");
                                let res = docker::delegator::delegate_docker_worker(&req.agent_yaml, &req.task).await;
                                skill_results.push(format!("[Docker Delegation Result: {}]", res));
                            }
                        }
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
