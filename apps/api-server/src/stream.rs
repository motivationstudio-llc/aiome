use axum::{
    extract::{State, Json},
    response::sse::{Event, KeepAlive, Sse},
};
use futures::stream::Stream;
use core::convert::Infallible;
use aiome_core::llm_provider::LlmProvider;
use aiome_core::traits::JobQueue;
use infrastructure::skills::UnverifiedSkill;
use std::sync::Arc;
use tokio::time::{timeout, Duration, interval};
use futures::StreamExt;
use tracing::info;

use crate::{AppState, AgentChatRequest};

pub async fn trigger_agent_chat_stream(
    State(state): State<AppState>,
    Json(payload): Json<AgentChatRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {

    let ollama_host = std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
    let ollama_model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5:0.5b".to_string());
    let provider = Arc::new(aiome_core::llm_provider::OllamaProvider::new(ollama_host, ollama_model));

    let stream = async_stream::stream! {
        // Discovery H: Guardrails check (Security Layer 0)
        if let shared::guardrails::ValidationResult::Blocked(reason) = shared::guardrails::validate_input(&payload.prompt) {
            yield Ok(Event::default().event("security_block").data(format!("🚨 [GUARDRAIL BLOCK] {}", reason)));
            return;
        }

        // Discovery B: Immune System check (Security Layer 1)
        let immune_system = infrastructure::immune_system::AdaptiveImmuneSystem::new(provider.clone());
        if let Ok(Some(rule)) = immune_system.verify_intent(&payload.prompt, state.job_queue.as_ref()).await {
            yield Ok(Event::default().event("security_block").data(format!("🚨 [SENTINEL BLOCK] {}\nPattern: {}", rule.action, rule.pattern)));
            return;
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

        let system_instructions = crate::build_system_instructions(&state, &karma_str);

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

                let calls = crate::parse_tool_calls(&full_reply);
                
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
                            let forge_future = crate::skill_handler::execute_forge_command(&skill_name, &skill_input, &state);
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
                            let out = crate::skill_handler::execute_wasm_skill(&skill_name, &skill_input, &state).await;
                            skill_results.push(out.clone());
                            let status = if out.contains("Error:") { "failed" } else { "Success" };
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

    Sse::new(stream).keep_alive(KeepAlive::default())
}
