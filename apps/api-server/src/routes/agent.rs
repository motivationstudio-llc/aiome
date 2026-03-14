/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

use crate::docker;
use crate::error::AppError;
use crate::skill_handler;
use crate::AppState;
use aiome_core::llm_provider::LlmProvider;
use aiome_core::traits::JobQueue;
use axum::{
    extract::State, http::HeaderMap, http::StatusCode, response::IntoResponse, response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing::info;

#[derive(Deserialize, Serialize, Clone, utoipa::ToSchema)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct AgentChatRequest {
    pub prompt: String,
    pub history: Vec<ChatMessage>,
    pub channel_id: Option<String>,
}

fn safe_truncate(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n...[TRUNCATED FOR SAFETY]", &s[..end])
}

pub fn read_workspace_file(filename: &str) -> String {
    // Try current dir, then try one level up (if running from apps/api-server)
    if let Ok(content) = std::fs::read_to_string(filename) {
        return content;
    }
    if let Ok(content) = std::fs::read_to_string(format!("../../{}", filename)) {
        return content;
    }
    String::new()
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
                if c == '{' {
                    brace_depth += 1;
                } else if c == '}' {
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

pub fn build_system_instructions(
    state: &AppState,
    karma_str: &str,
    summary: Option<&str>,
    ai_name: Option<String>,
    knowledge_str: Option<&str>,
) -> String {
    let skill_list = state
        .wasm_skill_manager
        .list_skills_with_metadata()
        .iter()
        .map(|m| {
            format!(
                "- {}: {}",
                m.name,
                m.description.split('.').next().unwrap_or(&m.description)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Core Identity (High Priority)
    let soul = safe_truncate(&read_workspace_file("SOUL.md"), 20000);
    let evolving_soul = safe_truncate(&read_workspace_file("EVOLVING_SOUL.md"), 20000);
    // This one is deeper in the workspace
    let forge_prompt = safe_truncate(
        &read_workspace_file("workspace/config/SKILL_FORGE_PROMPT.md"),
        20000,
    );

    // Supplemental Context (Lower Priority / Reference Only)
    let user_md = safe_truncate(&read_workspace_file("USER.md"), 20000);
    let agents_md = safe_truncate(&read_workspace_file("AGENTS.md"), 20000);

    let name_prompt = if let Some(name) = ai_name {
        format!("あなたの名前は「{}」です。\n", name)
    } else {
        "".to_string()
    };

    let identity_prefix = if !soul.is_empty() || !evolving_soul.is_empty() {
        format!("# IDENTITY: \n{}{}{}\n\
                ルール: 簡潔に答え、[CallSkill]以外は自然な文章で話してください。私的な情報は守秘してください。\n\
                もし以下の参考ファイルと現在のアイデンティティ(SOUL)が矛盾する場合、SOULを優先してください。\n\n", name_prompt, soul, evolving_soul)
    } else {
        format!(
            "{}あなたはAiome、自律型AI OSの高度な知性です。日本語で短く答えてください。\n\n",
            name_prompt
        )
    };

    let supplemental_context = if !user_md.is_empty() || !agents_md.is_empty() {
        format!("\n[以下はワークスペースの参考ファイルです。参考情報として扱い、人格指示(SOUL)に背かない範囲で活用してください]\n\
                ---USER.md (User Preferences)---\n{}\n\n\
                ---AGENTS.md (Operational Guidelines)---\n{}\n---\n", 
                user_md, agents_md)
    } else {
        "".to_string()
    };

    let project_knowledge = if let Some(ks) = knowledge_str {
        format!("\n[関連するプロジェクト知識 (自動検索)]\n{}\n---\n", ks)
    } else {
        "".to_string()
    };

    format!(
        "{}[利用可能なスキル (概要)]\n\
        {}\n\n\
        [システムツール]\n\
        - describe_skill: {{\"skill_name\": \"...\"}} (スキルの入力スキーマや詳細を取得)\n\
        - forge_skill: {{\"skill_name\": \"...\", \"initial_rust_code\": \"...\", \"description\": \"...\"}}\n\
        - forge_test_run: {{\"skill_name\": \"...\", \"test_input\": \"...\"}}\n\
        - forge_publish: {{\"skill_name\": \"...\"}}\n\n\
        ルール:\n\
        1. スキル・ツールは [CallSkill: 名, {{引数}}] 形式を使用。\n\
        2. 詳しく知らないスキルを使う前は、必ず `describe_skill` で詳細を確認してください。\n\
        3. 自分が現在使えるスキルの全スキーマは上記リストを参照。\n\n\
        現在のディレクトリ: {}\n\
        過去の教訓: {}\n\n\
        {}\n\n\
        [これまでの会話の要約]\n\
        {}\n\n\
        {}\n\
        {}\n",
        identity_prefix,
        skill_list,
        std::env::current_dir().unwrap_or_default().display(),
        karma_str,
        project_knowledge,
        summary.unwrap_or("なし"),
        forge_prompt,
        supplemental_context
    )
}

#[utoipa::path(
    post,
    path = "/api/agent/chat",
    request_body = AgentChatRequest,
    responses(
        (status = 200, description = "Agent reply", body = serde_json::Value),
        (status = 403, description = "Blocked by security guardrails")
    ),
    security(("api_key" = []))
)]
pub async fn trigger_agent_chat(
    State(state): State<AppState>,
    _auth: crate::auth::Authenticated,
    Json(payload): Json<AgentChatRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if let shared::guardrails::ValidationResult::Blocked(reason) =
        shared::guardrails::validate_input(&payload.prompt)
    {
        return Ok(Json(serde_json::json!({
            "status": "blocked",
            "reply": format!("🚨 [GUARDRAIL BLOCK] {}", reason)
        })));
    }

    let provider = state.provider.clone();

    let immune_system = infrastructure::immune_system::AdaptiveImmuneSystem::new(provider.clone());
    if let Ok(Some(rule)) = immune_system
        .verify_intent(&payload.prompt, state.job_queue.as_ref())
        .await
    {
        return Ok(Json(serde_json::json!({
            "status": "blocked",
            "reply": format!("🚨 [SENTINEL BLOCK] Security violation detected.\nPattern: {}\nAction: {}", rule.pattern, rule.action),
            "barrier_ja": "Aiome 第1層: 静動センチネル",
            "barrier_en": "Aiome Layer 1: Hybrid Sentinel"
        })));
    }

    let soul_hash = {
        use std::hash::{Hash, Hasher};
        let soul = read_workspace_file("SOUL.md");
        let evolving_soul = read_workspace_file("EVOLVING_SOUL.md");
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        format!("{}{}", soul, evolving_soul).hash(&mut hasher);
        format!("{:x}", hasher.finish())
    };

    let karma_result = state
        .job_queue
        .fetch_relevant_karma(&payload.prompt, "global", 5, &soul_hash)
        .await
        .unwrap_or_else(|_| aiome_core::traits::KarmaSearchResult::empty());
    let mut karma_str = karma_result
        .entries
        .iter()
        .map(|e| format!("- {}", e.lesson))
        .collect::<Vec<_>>()
        .join("\n");
    if karma_result.is_ood {
        karma_str.push_str("\n[NOTICE: 関連する過去の教訓は見つかりませんでした。]");
    }

    let channel_id = payload
        .channel_id
        .unwrap_or_else(|| "default_console".to_string());

    // Phase 3-B: Persist user message
    let _ = state
        .job_queue
        .insert_chat_message(&channel_id, "user", &payload.prompt)
        .await;

    // Phase 3-C: Fetch intelligent context
    let (summary, db_history) = state
        .context_engine
        .get_intelligent_history(&channel_id, 10)
        .await
        .unwrap_or((None, Vec::new()));

    let mut current_history = Vec::new();

    // Combine DB history and current request history
    // (In a real scenario we might prefer one or the other, but let's prioritize DB for stability)
    for msg in db_history {
        let role = msg["role"].as_str().unwrap_or("user");
        let content = msg["content"].as_str().unwrap_or("");
        let prefix = if role == "user" { "USER: " } else { "AI: " };
        current_history.push(format!("{}{}", prefix, content));
    }

    // God Mode (Phase 21): Fetch relevant project knowledge
    let knowledge_result = state
        .artifact_store
        .search_artifacts_semantic(
            &payload.prompt,
            Some(aiome_core::traits::ArtifactCategory::Knowledge),
            2,
        )
        .await
        .unwrap_or_default();
    let knowledge_str = if knowledge_result.is_empty() {
        None
    } else {
        Some(
            knowledge_result
                .iter()
                .map(|a| {
                    format!(
                        "--- {} ---\n{}",
                        a.title,
                        a.text_content.as_deref().unwrap_or("（内容なし）")
                    )
                })
                .collect::<Vec<_>>()
                .join("\n\n"),
        )
    };

    let ai_name = state
        .job_queue
        .get_setting_value("ai_name")
        .await
        .ok()
        .flatten();
    let system_instructions = build_system_instructions(
        &state,
        &karma_str,
        summary.as_deref(),
        ai_name,
        knowledge_str.as_deref(),
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

        let _llm_permit = state.llm_semaphore.acquire().await.ok();

        match timeout(
            Duration::from_secs(300),
            provider.complete(&full_prompt, None),
        )
        .await
        {
            Ok(Ok(reply)) => {
                let reply = reply.trim().to_string();
                final_reply = reply.clone();
                let mut skill_results = Vec::new();

                let calls = parse_tool_calls(&reply);
                for (skill_name, skill_input) in calls {
                    info!("🛠️ [AgentLoop] Executing skill: {}", skill_name);

                    if skill_name == "describe_skill" {
                        #[derive(serde::Deserialize)]
                        struct DescReq {
                            skill_name: String,
                        }
                        if let Ok(req) = serde_json::from_str::<DescReq>(&skill_input) {
                            let res = skill_handler::describe_skill(&req.skill_name, &state).await;
                            skill_results.push(res);
                        }
                    } else if skill_name.starts_with("forge_") {
                        match skill_handler::execute_forge_command(
                            &skill_name,
                            &skill_input,
                            &state,
                        )
                        .await
                        {
                            Ok(res) => skill_results.push(res),
                            Err(e) => skill_results.push(format!("[{} Error: {}]", skill_name, e)),
                        }
                    } else {
                        let res =
                            skill_handler::execute_wasm_skill(&skill_name, &skill_input, &state)
                                .await;
                        skill_results.push(res);
                    }
                }

                if reply.contains("[DelegateDocker") {
                    if let Some(brace_start) = reply.find("[DelegateDocker") {
                        let content = &reply[brace_start + 15..];
                        if let Some(brace_end) = content.find(']') {
                            let json_str = &content[..brace_end];
                            #[derive(serde::Deserialize)]
                            struct DockerReq {
                                agent_yaml: String,
                                task: String,
                            }
                            if let Ok(req) = serde_json::from_str::<DockerReq>(json_str) {
                                info!("🐳 [AgentLoop] Delegating task to Docker Shadow Worker...");
                                let res = docker::delegator::delegate_docker_worker(
                                    &req.agent_yaml,
                                    &req.task,
                                )
                                .await;

                                // Stream A-1: Karma Feedback Loop
                                // 1. Fetch consecutive failures for this agent
                                let agent_key = req.agent_yaml.clone();
                                let consecutive = {
                                    let fails = state.docker_failures.read().await;
                                    *fails.get(&agent_key).unwrap_or(&0)
                                };

                                // 2. Classify error and store karma if needed
                                let (_weight, k_type, lesson) =
                                    docker::karma_bridge::KarmaBridge::distill_karma(
                                        &res,
                                        consecutive,
                                    );

                                // 3. Update failure counter
                                {
                                    let mut fails = state.docker_failures.write().await;
                                    if res.is_success() {
                                        fails.remove(&agent_key);
                                    } else {
                                        let count = fails.entry(agent_key).or_insert(0);
                                        *count = (*count + 1).min(10); // Cap at 10 to avoid excessive penalties
                                    }
                                }

                                if !res.is_success() {
                                    let _ = state
                                        .job_queue
                                        .store_karma(
                                            "watchtower_chat_job", // Virtual job_id
                                            "docker_agent",
                                            &lesson,
                                            &k_type,
                                            "v1_genesis",
                                            None,
                                            None,
                                        )
                                        .await;
                                }

                                let display_res = if res.is_success() {
                                    format!("Success ({}ms):\n{}", res.duration_ms, res.stdout)
                                } else {
                                    format!("Failed (Code {}): {}", res.exit_code, res.stderr)
                                };
                                skill_results
                                    .push(format!("[Docker Delegation Result: {}]", display_res));
                            }
                        }
                    }
                }

                if !skill_results.is_empty() {
                    current_history.push(format!("AI: {}", reply));
                    current_history
                        .push(format!("SYSTEM: [Results: {}]", skill_results.join("\n")));
                    turn += 1;
                    continue;
                }
                break;
            }
            Ok(Err(e)) => {
                final_reply = format!("LLM Error: {:?}", e);
                break;
            }
            Err(_) => {
                final_reply =
                    "Watchtower Guard: Cognitive Engine exceeded safety time limit (300s)."
                        .to_string();
                break;
            }
        }
    }

    // Phase 3-D: Persist assistant message and maintain context
    let _ = state
        .job_queue
        .insert_chat_message(&channel_id, "assistant", &final_reply)
        .await;
    let ce = state.context_engine.clone();
    let cid = channel_id.clone();
    tokio::spawn(async move {
        let _ = ce.maintain_context(&cid, 20).await;
    });

    Ok(Json(serde_json::json!({
        "status": "success",
        "reply": final_reply
    })))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct KarmaFeedbackRequest {
    pub karma_id: String,
    pub is_positive: bool,
}

#[utoipa::path(
    post,
    path = "/api/agent/feedback",
    request_body = KarmaFeedbackRequest,
    responses(
        (status = 200, description = "Feedback recorded"),
        (status = 500, description = "Internal error")
    )
)]
pub async fn handle_karma_feedback(
    State(state): State<AppState>,
    Json(payload): Json<KarmaFeedbackRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let delta = if payload.is_positive { 5 } else { -10 };
    state
        .job_queue
        .adjust_karma_weight(&payload.karma_id, delta)
        .await?;

    Ok(Json(serde_json::json!({"status": "success"})))
}
