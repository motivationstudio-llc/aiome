/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

use crate::AppState;
use aiome_core::traits::JobQueue;
use infrastructure::skills::UnverifiedSkill;
use tracing::info;

pub async fn execute_forge_command(
    command: &str,
    payload: &str,
    state: &AppState,
) -> Result<String, String> {
    // Acquire forge semaphore to prevent concurrent builds (Phase 13-A)
    let _permit = state
        .forge_semaphore
        .acquire()
        .await
        .map_err(|e| format!("Failed to acquire forge semaphore: {}", e))?;

    match command {
        "forge_skill" => {
            #[derive(serde::Deserialize)]
            struct ForgeReq {
                skill_name: String,
                initial_rust_code: String,
                description: String,
            }
            match serde_json::from_str::<ForgeReq>(payload) {
                Ok(req) => {
                    // Security Gate: Validate skill name (Discovery A)
                    if req.skill_name.contains("..")
                        || req.skill_name.contains('/')
                        || req.skill_name.contains('\\')
                    {
                        return Err(
                            "Security Violation: Invalid skill name (path traversal detected)"
                                .into(),
                        );
                    }

                    match state
                        .skill_forge
                        .forge_skill(&req.skill_name, &req.initial_rust_code, 3, &req.description)
                        .await
                    {
                        Ok(_) => Ok(format!(
                            "[forge_skill Result: Successfully compiled {}.wasm]",
                            req.skill_name
                        )),
                        Err(e) => {
                            let err_msg = format!("Forge failed: {}", e);
                            state
                                .job_queue
                                .store_karma(
                                    "forge",
                                    &req.skill_name,
                                    &err_msg,
                                    "failure",
                                    "current",
                                    None,
                                    None,
                                )
                                .await
                                .ok();
                            Err(err_msg)
                        }
                    }
                }
                Err(e) => Err(format!("Invalid JSON - {}", e)),
            }
        }
        "forge_test_run" => {
            #[derive(serde::Deserialize)]
            struct TestReq {
                skill_name: String,
                test_input: String,
            }
            match serde_json::from_str::<TestReq>(payload) {
                Ok(req) => {
                    match state
                        .wasm_skill_manager
                        .dry_run_skill(&req.skill_name, &req.test_input)
                        .await
                    {
                        Ok(true) => {
                            Ok("[forge_test_run Result: Skill verified successfully]".into())
                        }
                        Ok(false) => {
                            Err("Skill failed security checklist (OOM or Format error)".into())
                        }
                        Err(e) => Err(format!("Test error: {}", e)),
                    }
                }
                Err(e) => Err(format!("Invalid JSON - {}", e)),
            }
        }
        "forge_publish" => {
            #[derive(serde::Deserialize)]
            struct PublishReq {
                skill_name: String,
            }
            match serde_json::from_str::<PublishReq>(payload) {
                Ok(req) => {
                    // Security Gate: Validate skill name (Discovery A)
                    if req.skill_name.contains("..")
                        || req.skill_name.contains('/')
                        || req.skill_name.contains('\\')
                    {
                        return Err(
                            "Security Violation: Invalid skill name (path traversal detected)"
                                .into(),
                        );
                    }

                    // Security Gate: Use deliver_output for safe atomic moves (Sec-1)
                    let source = std::path::Path::new("workspace/skills/custom")
                        .join(format!("{}.wasm", req.skill_name));
                    let meta_src = std::path::Path::new("workspace/skills/custom")
                        .join(format!("{}.meta.json", req.skill_name));

                    if source.exists() {
                        // Deliver WASM
                        match infrastructure::workspace_manager::WorkspaceManager::deliver_output(
                            "forge_publish",
                            &source,
                            "workspace/skills",
                        )
                        .await
                        {
                            Ok(dest_path) => {
                                // Deliver Metadata (Simple copy for now as deliver_output adds prefix which we don't want for meta yet)
                                // Ideally WasmSkillManager would handle prefixed names.
                                let meta_dest = std::path::Path::new("workspace/skills")
                                    .join(format!("{}.meta.json", req.skill_name));
                                let _ = std::fs::copy(meta_src, meta_dest);

                                state.wasm_skill_manager.invalidate_cache(&req.skill_name);
                                state.wasm_skill_manager.hot_reload_skills();

                                info!(
                                    "✅ [SkillForge] Skill {} published successfully to {}",
                                    req.skill_name,
                                    dest_path.display()
                                );
                                Ok(format!(
                                    "[forge_publish Result: Skill {} published as {}]",
                                    req.skill_name,
                                    dest_path.file_name().unwrap_or_default().to_string_lossy()
                                ))
                            }
                            Err(e) => Err(format!("Publish failed during delivery: {}", e)),
                        }
                    } else {
                        Err("Source WASM not found. Did you forge it first?".into())
                    }
                }
                Err(e) => Err(format!("Invalid JSON - {}", e)),
            }
        }
        _ => Err(format!("Unknown forge command: {}", command)),
    }
}

pub async fn execute_wasm_skill(skill_name: &str, skill_input: &str, state: &AppState) -> String {
    let test_payload = state
        .wasm_skill_manager
        .get_metadata(skill_name)
        .and_then(|m| m.inputs.first().cloned())
        .unwrap_or_else(|| "{}".to_string());

    let unverified = UnverifiedSkill {
        name: skill_name.to_string(),
        input_test_payload: test_payload,
    };

    match unverified.verify(&state.wasm_skill_manager).await {
        Ok(v) => {
            match state
                .wasm_skill_manager
                .call_skill(&v, "call", skill_input, None)
                .await
            {
                Ok(res) => {
                    let limited_res = if res.len() > 3000 {
                        format!("{}... [Truncated for brevity]", &res[..3000])
                    } else {
                        res
                    };
                    format!("[{} Result: {}]", skill_name, limited_res)
                }
                Err(e) => {
                    format!("[{} Error: {}]", skill_name, e)
                }
            }
        }
        Err(e) => {
            format!(
                "[{} Error: Verification failed or Skill not found: {}]",
                skill_name, e
            )
        }
    }
}

pub async fn describe_skill(skill_name: &str, state: &AppState) -> String {
    if let Some(meta) = state.wasm_skill_manager.get_metadata(skill_name) {
        format!(
            "[Detail for {}]\nDescription: {}\nOperations: {:?}\nInput Schema: {:?}\nPermissions: {:?}",
            skill_name, meta.description, meta.capabilities, meta.inputs, meta.permissions
        )
    } else {
        format!(
            "[Skill {} not found or has no detailed metadata]",
            skill_name
        )
    }
}
