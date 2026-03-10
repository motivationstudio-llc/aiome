use axum::{
    extract::{State, Json},
    response::sse::{Event, KeepAlive, Sse},
    response::IntoResponse,
};
use futures::stream::Stream;
use core::convert::Infallible;
use aiome_core::llm_provider::LlmProvider;
use aiome_core::traits::JobQueue;
// use infrastructure::skills::UnverifiedSkill;
use std::sync::Arc;
use tokio::time::{timeout, Duration, interval};
use futures::StreamExt;
use tracing::info;
use crate::skill_handler;

use crate::AppState;
use crate::routes::agent::{AgentChatRequest, build_system_instructions, parse_tool_calls};

pub async fn trigger_agent_chat_stream(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<AgentChatRequest>,
) -> impl axum::response::IntoResponse {
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
            axum::http::StatusCode::UNAUTHORIZED, 
            "Unauthorized"
        ).into_response();
    }

    let ollama_host = std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
    let ollama_model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5:0.5b".to_string());
    let provider = Arc::new(aiome_core::llm_provider::OllamaProvider::new(ollama_host, ollama_model));

    let stream = async_stream::stream! {
        // Discovery H: Guardrails check (Security Layer 0)
        if let shared::guardrails::ValidationResult::Blocked(reason) = shared::guardrails::validate_input(&payload.prompt) {
            yield Ok::<Event, Infallible>(Event::default().event("security_block").data(format!("🚨 [GUARDRAIL BLOCK] {}", reason)));
            return;
        }

        // Discovery B: Immune System check (Security Layer 1)
        let immune_system = infrastructure::immune_system::AdaptiveImmuneSystem::new(provider.clone());
        if let Ok(Some(rule)) = immune_system.verify_intent(&payload.prompt, state.job_queue.as_ref()).await {
            let stats = state.job_queue.get_agent_stats().await.unwrap_or_default();
            let _ = state.job_queue.record_evolution_event(
                stats.level, 
                "ImmuneAlert", 
                &format!("Block: {} (Pattern: {})", rule.action, rule.pattern),
                None, 
                None
            ).await;
            yield Ok::<Event, Infallible>(Event::default().event("security_block").data(format!("🚨 [SENTINEL BLOCK] {}\nPattern: {}", rule.action, rule.pattern)));
            return;
        }

        let soul = std::fs::read_to_string("SOUL.md").unwrap_or_default();
        let evolving_soul = std::fs::read_to_string("EVOLVING_SOUL.md").unwrap_or_default();
        let soul_hash = {
            let mut h: u64 = 0;
            for b in format!("{}{}", soul, evolving_soul).as_bytes() {
                h = h.wrapping_add(*b as u64).wrapping_mul(31);
            }
            format!("{:x}", h)
        };

        // Sprint 1-A: Fetch relevant karma using proper search
        let karma_result = state.job_queue.fetch_relevant_karma(&payload.prompt, "global", 5, &soul_hash).await.unwrap_or_else(|_| aiome_core::traits::KarmaSearchResult::empty());
        
        let mut karma_str = karma_result.entries.iter()
            .map(|e| format!("- {}", e.lesson))
            .collect::<Vec<_>>()
            .join("\n");

        if karma_result.is_ood {
            karma_str.push_str("\n[NOTICE: 関連する過去の教訓は見つかりませんでした。このリクエストは新しいコンテキストである可能性があります。]");
        }

        // Notify client about relevant karma being used (Sprint 4-A)
        yield Ok::<Event, Infallible>(Event::default().event("karma").data(&karma_str));

        let history_len = payload.history.len();
        let start_idx = if history_len > 10 { history_len - 10 } else { 0 };
        
        let mut current_history = Vec::new();
        for msg in &payload.history[start_idx..] {
            let prefix = if msg.role == "user" { "USER: " } else { "AI: " };
            current_history.push(format!("{}{}", prefix, msg.content));
        }

        let system_instructions = build_system_instructions(&state, &karma_str);

        let max_turns = 15;
        let mut turn = 0;
        let original_prompt = payload.prompt.clone();

        while turn < max_turns {
            let full_prompt = format!(
                "{}\n{}\nUSER: {}\nAI: ", 
                system_instructions, 
                current_history.join("\n"),
                original_prompt
            );

            yield Ok(Event::default().event("turn_start").data(turn.to_string()));

            // Phase 13-A: Acquire LLM semaphore
            let _llm_permit = state.llm_semaphore.acquire().await.ok();

            if let Ok(Ok(mut llm_stream)) = timeout(Duration::from_secs(300), provider.stream_complete(&full_prompt, None)).await {
                
                let mut buffer = String::new();
                let mut full_reply = String::new();
                let mut is_tool_call_mode = false;

                while let Some(chunk_res) = llm_stream.next().await {
                    let chunk: String = chunk_res.unwrap_or_default();
                    full_reply.push_str(&chunk);

                    if !is_tool_call_mode {
                        buffer.push_str(&chunk);
                        if let Some(idx) = buffer.find("[CallSkill") {
                            let text = buffer[..idx].to_string();
                            if !text.is_empty() {
                                yield Ok(Event::default().event("text").data(&text));
                            }
                            is_tool_call_mode = true;
                            yield Ok(Event::default().event("tool_detect").data("tool detected"));
                        } else if let Some(idx) = buffer.rfind('[') {
                            let text = buffer[..idx].to_string();
                            if !text.is_empty() {
                                yield Ok(Event::default().event("text").data(&text));
                            }
                            buffer = buffer[idx..].to_string();
                        } else {
                            if !buffer.is_empty() {
                                yield Ok(Event::default().event("text").data(&buffer));
                                buffer.clear();
                            }
                        }
                    }
                }
                
                if !is_tool_call_mode && !buffer.is_empty() {
                    yield Ok(Event::default().event("text").data(&buffer));
                }

                let calls = parse_tool_calls(&full_reply);
                
                if calls.is_empty() {
                    yield Ok(Event::default().event("turn_end").data("done"));
                    break;
                } else {
                    let mut skill_results = Vec::new();
                    for (skill_name, skill_input) in calls {
                        info!("🛠️ [AgentStreamLoop] Executing skill: {}", skill_name);
                        yield Ok(Event::default().event("tool_exec").data(format!("Executing {}", skill_name)));

                        if skill_name.starts_with("forge_") {
                            // Phase 12-C: SSE Heartbeat implementation for long-running forge tasks
                            let mut heartbeat_ticker = interval(Duration::from_secs(5));
                            let forge_future = skill_handler::execute_forge_command(&skill_name, &skill_input, &state);
                            tokio::pin!(forge_future);

                            loop {
                                tokio::select! {
                                    _ = heartbeat_ticker.tick() => {
                                        yield Ok(Event::default().event("heartbeat").data("build in progress..."));
                                    }
                                    res = &mut forge_future => {
                                        match res {
                                            Ok(out) => {
                                                skill_results.push(out.clone());
                                                yield Ok(Event::default().event("tool_result").data(out));
                                            }
                                            Err(e) => {
                                                skill_results.push(format!("[{} Error: {}]", skill_name, e));
                                                yield Ok(Event::default().event("tool_result").data(format!("Error: {}", e)));
                                            }
                                        }
                                        break;
                                    }
                                }
                            }
                        } else {
                            let out = skill_handler::execute_wasm_skill(&skill_name, &skill_input, &state).await;
                            
                            // Phase 2B: Record skill execution
                            let stats = state.job_queue.get_agent_stats().await.unwrap_or_default();
                            let status = if out.contains("Error:") { "failed" } else { "success" };
                            let _ = state.job_queue.record_evolution_event(
                                stats.level,
                                "SkillExecution",
                                &format!("Exec: {} -> {}", skill_name, status),
                                Some(&skill_name),
                                None
                            ).await;

                            skill_results.push(out.clone());
                            yield Ok(Event::default().event("tool_result").data(format!("{}: {}", skill_name, status)));
                        }
                    }

                    current_history.push(format!("AI: {}", full_reply));
                    current_history.push(format!("SYSTEM: [Results: {}]", skill_results.join("\n")));
                    turn += 1;
                }
            } else {
                yield Ok(Event::default().event("error").data("LLM timeout or error"));
                break;
            }
        }
        yield Ok(Event::default().event("done").data("stream finished"));
    };

    Sse::new(stream).keep_alive(KeepAlive::default()).into_response()
}

pub async fn trigger_system_vitality_stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        let mut interval = interval(Duration::from_secs(5));
        let mut last_karma_count = 0;
        let mut last_evolution_count = 0;
        let mut last_level = 0;
        let mut last_is_thinking = false;

        // Initialize state
        if let Ok(stats) = state.job_queue.get_agent_stats().await {
            last_level = stats.level;
            last_karma_count = state.job_queue.fetch_all_karma(100).await.unwrap_or_default().len();
            last_evolution_count = state.job_queue.fetch_evolution_history(100).await.unwrap_or_default().len();
            last_is_thinking = state.job_queue.get_pending_job_count().await.unwrap_or(0) > 0;
        }

        loop {
            interval.tick().await;

            // 4. Thinking Check
            if let Ok(pending_count) = state.job_queue.get_pending_job_count().await {
                let is_thinking = pending_count > 0;
                if is_thinking && !last_is_thinking {
                    yield Ok(Event::default().event("job_started").data("thinking"));
                } else if !is_thinking && last_is_thinking {
                    yield Ok(Event::default().event("job_completed").data("idle"));
                }
                last_is_thinking = is_thinking;
            }

            if let Ok(stats) = state.job_queue.get_agent_stats().await {
                // 1. Level Up Check
                if stats.level > last_level {
                    yield Ok(Event::default().event("level_up").data(serde_json::to_string(&stats).unwrap_or_default()));
                    last_level = stats.level;
                }

                // 2. Karma Check
                let current_karmas = state.job_queue.fetch_all_karma(100).await.unwrap_or_default();
                if current_karmas.len() > last_karma_count {
                    // Send newest karma
                    if let Some(new_karma) = current_karmas.first() {
                         yield Ok(Event::default().event("karma_update").data(serde_json::to_string(new_karma).unwrap_or_default()));
                    }
                    last_karma_count = current_karmas.len();
                }

                // 3. Evolution Check (Inspiration / Soul Mutation / Alerts / SkillExec)
                let current_evos = state.job_queue.fetch_evolution_history(100).await.unwrap_or_default();
                if current_evos.len() > last_evolution_count {
                    if let Some(new_evo) = current_evos.first() {
                        let event_type = new_evo["event_type"].as_str().unwrap_or("inspiration");
                        let sse_event = match event_type {
                            "ImmuneAlert" => "immune_alert",
                            "SkillExecution" => "skill_execution",
                            _ => "inspiration",
                        };
                        yield Ok(Event::default().event(sse_event).data(serde_json::to_string(new_evo).unwrap_or_default()));
                    }
                    last_evolution_count = current_evos.len();
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}
