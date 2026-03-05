/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

use std::path::PathBuf;
use tokio::fs;
use rig::providers::gemini;
use rig::client::CompletionClient;
use rig::completion::Prompt;
use factory_core::traits::JobQueue;

use tracing::{info, warn, error};

pub struct SoulMutator {
    gemini_api_key: String,
    model_name: String,
}

impl SoulMutator {
    pub fn new(gemini_api_key: &str, model_name: &str, _workspace_dir: PathBuf) -> Self {
        Self {
            gemini_api_key: gemini_api_key.to_string(),
            model_name: model_name.to_string(),
        }
    }

    /// 魂の変異（Transmigration）を試行する。
    /// 指定された Karma 蓄積量や、特定のマイルストーン達成時に呼び出すことを想定。
    pub async fn transmute(&self, job_queue: &dyn JobQueue) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        info!("🧬 [SoulMutator] Starting Transmigration phase...");

        let root_dir = std::env::current_dir()?;
        let soul_path = root_dir.join("SOUL.md");
        let evolving_soul_path = root_dir.join("EVOLVING_SOUL.md");

        // 1. Load Current Soul State
        let master_soul = fs::read_to_string(&soul_path).await
            .map_err(|e| format!("Failed to read SOUL.md: {}", e))?;
        let current_evolving_soul = fs::read_to_string(&evolving_soul_path).await
            .map_err(|e| format!("Failed to read EVOLVING_SOUL.md: {}", e))?;

        // 1. Fetch unincorporated karma (Lessons learned under current/old soul)
        let current_soul_content = fs::read_to_string(&evolving_soul_path).await
            .map_err(|e| format!("Failed to read EVOLVING_SOUL.md: {}", e))?;
        let current_hash = self.compute_hash(&current_soul_content);

        let recent_karmas = job_queue.fetch_unincorporated_karma(30, &current_hash).await?;
        let top_jobs = job_queue.fetch_top_performing_jobs(5).await?;

        if recent_karmas.len() < 10 && top_jobs.is_empty() {
            warn!("🧬 [SoulMutator] Not enough new karma ({} < 10) to justify mutation. Skipping.", recent_karmas.len());
            return Ok(false);
        }

        // 3. Construct Analysis Context
        let karma_text = serde_json::to_string_pretty(&recent_karmas)?;
        let top_jobs_text: Vec<String> = top_jobs.iter()
            .map(|j| format!("- Topic: '{}', Style: '{}'", j.topic, j.style))
            .collect();

        // 4. Call LLM for Mutation Proposal
        let client = gemini::Client::new(&self.gemini_api_key)
            .map_err(|e| format!("Gemini Client init failed: {}", e))?;

        let preamble = format!(
            "あなたは自己進化するAI「Aiome」の深層心理再構築モジュールです。
\
             以下の【不変の法（SOUL.md）】を厳守した上で、最近の【実績・教訓】を分析し、
\
             自身の【進化する自我（EVOLVING_SOUL.md）】を最新の状態にアップデートしてください。

\
             ## 不変の法 (SOUL.md)
\
             {}

\
             ## 最近の実績・教訓
\
             ### 成功事例
\
             {}
\
             ### 抽出された Karma (直近20件)
\
             {}

\
             ## 指示
\
             1. 現在の EVOLVING_SOUL.md の内容を土台とし、新たな洞察、価値観、マスターへの理解、改善された稼働方針を反映させてください。
\
             2. フォーマットは現在の EVOLVING_SOUL.md を踏襲し、Markdown形式で出力してください。
\
             3. SOUL.md の『不変の戒律』を絶対に書き換えないこと（EVOLVING_SOULのみを対象とする）。
\
             4. 文字数は1500文字程度に収めること。
\
             5. 出力は純粋なMarkdownのみとし、前置きは不要。",
            master_soul,
            top_jobs_text.join("
"),
            karma_text
        );

        let agent = client.agent(&self.model_name).preamble(&preamble).build();
        let prompt = format!("現在のあなたの進化状況を反映した、最新の EVOLVING_SOUL.md を生成せよ。

現在の内容:
{}", current_evolving_soul);

        match agent.prompt(prompt).await {
            Ok(mut new_soul_content) => {
                // Remove ```markdown wrapping if the LLM included it
                if new_soul_content.starts_with("```markdown") {
                    new_soul_content = new_soul_content.trim_start_matches("```markdown").trim().to_string();
                } else if new_soul_content.starts_with("```") {
                    new_soul_content = new_soul_content.trim_start_matches("```").trim().to_string();
                }
                if new_soul_content.ends_with("```") {
                    new_soul_content = new_soul_content.trim_end_matches("```").trim().to_string();
                }

                let old_hash = self.compute_hash(&current_evolving_soul);
                let new_hash = self.compute_hash(&new_soul_content);

                if old_hash == new_hash {
                    info!("🧬 [SoulMutator] Mutation resulted in no change. Staying in current state.");
                    return Ok(false);
                }

                // 5. Atomic Update with History
                info!("🧬 [SoulMutator] Mutation detected. New Hash: {}", new_hash);
                
                // Backup for safety
                let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
                let backup_path = evolving_soul_path.with_extension(format!("bak.{}", timestamp));
                let _ = fs::copy(&evolving_soul_path, &backup_path).await;

                // Write new soul
                fs::write(&evolving_soul_path, &new_soul_content).await
                    .map_err(|e| format!("Failed to write EVOLVING_SOUL.md: {}", e))?;

                // 2. Git Commit (Self-Traceability)
                let mutation_reason = format!("Transmigration based on {} unincorporated lessons.", recent_karmas.len());
                let _ = std::process::Command::new("git")
                    .arg("add")
                    .arg(&evolving_soul_path)
                    .output();
                let _ = std::process::Command::new("git")
                    .arg("commit")
                    .arg("-m")
                    .arg(format!("🧬 [AI Self-Evolution] {}", mutation_reason))
                    .output();

                // Record history
                let mutation_reason = format!("Transmigration based on {} unincorporated lessons.", recent_karmas.len());
                if let Err(e) = job_queue.record_soul_mutation(&old_hash, &new_hash, &mutation_reason).await {
                    error!("❌ [SoulMutator] Failed to record mutation history: {}", e);
                }

                // 6. Mark karma as incorporated into the new soul
                let karma_ids: Vec<String> = recent_karmas.iter()
                    .map(|k| k["id"].as_str().unwrap_or_default().to_string())
                    .filter(|id| !id.is_empty())
                    .collect();
                
                if let Err(e) = job_queue.mark_karma_as_incorporated(karma_ids, &new_hash).await {
                    error!("❌ [SoulMutator] Failed to mark karma as incorporated: {}", e);
                }

                info!("✅ [SoulMutator] Transmigration complete. I have evolved.");
                Ok(true)
            }
            Err(e) => {
                error!("❌ [SoulMutator] LLM Mutation failed: {}", e);
                Err(e.into())
            }
        }
    }

    fn compute_hash(&self, content: &str) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:16x}", hasher.finish())
    }
}
