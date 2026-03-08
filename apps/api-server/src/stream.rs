use axum::{
    extract::{State, Json},
    response::sse::{Event, KeepAlive, Sse},
};
use futures::stream::Stream;
use core::convert::Infallible;
use aiome_core::llm_provider::LlmProvider;
use aiome_core::traits::JobQueue;
use aiome_core::error::AiomeError;
use infrastructure::skills::UnverifiedSkill;
use std::sync::Arc;
use tokio::time::{timeout, Duration};
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
        ルール: スキルは [CallSkill] 形式を使用。1ターン1アクション。\n\
        現在のディレクトリ: {}\n\
        過去の教訓: {}\n",
        std::env::current_dir().unwrap_or_default().display(),
        karma_str
    );

    let max_turns = 15;
    let mut turn = 0;
    let original_prompt = payload.prompt.clone();

    let stream = async_stream::stream! {
        while turn < max_turns {
            let full_prompt = format!(
                "{}\n{}\nUSER: {}\nAI: ", 
                system_instructions, 
                current_history.join("\n"),
                original_prompt
            );

            yield Ok(Event::default().event("turn_start").data(turn.to_string()));

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
                                yield Ok(Event::default().event("tool_result").data(format!("{}: Success", skill_name)));
                            } else {
                                skill_results.push(format!("[{} Error: Execution failed]", skill_name));
                                yield Ok(Event::default().event("tool_result").data(format!("{}: Execution failed", skill_name)));
                            }
                        } else {
                            skill_results.push(format!("[{} Error: Verification failed or Skill not found]", skill_name));
                            yield Ok(Event::default().event("tool_result").data(format!("{}: Verification failed", skill_name)));
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
