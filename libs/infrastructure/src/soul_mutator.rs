/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 * You may not provide the software to third parties as a hosted or managed service, 
 * where the service provides users with access to any substantial set of the features 
 * or functionality of the software.
 */

use std::path::PathBuf;
use tokio::fs;
use aiome_core::traits::JobQueue;
use aiome_core::llm_provider::LlmProvider;
use std::sync::Arc;
use tracing::{info, error};

pub struct SoulMutator {
    provider: Arc<dyn LlmProvider>,
}

impl SoulMutator {
    pub fn new(provider: Arc<dyn LlmProvider>, _workspace_dir: PathBuf) -> Self {
        Self { provider }
    }

    /// 魂の変異（Transmigration）を試行する。
    pub async fn transmute(&self, job_queue: &dyn JobQueue) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        info!("🧬 [SoulMutator] Starting Transmigration phase using {}...", self.provider.name());

        let root_dir = std::env::current_dir()?;
        let soul_path = root_dir.join("SOUL.md");
        let evolving_soul_path = root_dir.join("EVOLVING_SOUL.md");

        if !soul_path.exists() || !evolving_soul_path.exists() {
            return Err("SOUL.md or EVOLVING_SOUL.md not found. Transmutation impossible.".into());
        }

        // 1. Read Current Soul State
        let master_soul = fs::read_to_string(&soul_path).await?;
        let current_evolving_soul = fs::read_to_string(&evolving_soul_path).await?;

        // 2. Collect High-Karma Lessons
        let top_jobs = job_queue.fetch_relevant_karma("excellent", "global", 5, "current").await
            .map_err(|e| format!("Failed to fetch karma: {}", e))?;
        
        let mut top_jobs_text = Vec::new();
        for lesson in top_jobs {
            top_jobs_text.push(format!("- {}", lesson));
        }

        if top_jobs_text.is_empty() {
             info!("🧬 [SoulMutator] Not enough high-quality Karma accumulated yet. Skipping mutation.");
             return Ok(false);
        }

        // 3. Collect Technical failures and feedback
        let failures = job_queue.fetch_relevant_karma("failure error", "global", 3, "current").await
            .map_err(|e| format!("Failed to fetch failures: {}", e))?;
        
        let mut karma_text = String::new();
        for f in failures {
            karma_text.push_str(&format!("- [FAILURE LOG]: {}\n", f));
        }

        // 4. Mutation Loop
        let preamble = format!(
            "AI の魂の進化プロセスの継続。EVOLVING_SOUL.md を更新せよ。\n\nユーザーソウル:\n{}\n\n成功体験:\n{}\n\n最近の課題:\n{}",
            master_soul,
            top_jobs_text.join("\n"),
            karma_text
        );

        let prompt_text = format!("現在のあなたの進化状況を反映した、最新の EVOLVING_SOUL.md を生成せよ。\n\n現在の内容:\n{}", current_evolving_soul);

        let response = self.provider.complete(&prompt_text, Some(&preamble)).await
            .map_err(|e| format!("Mutation LLM failed: {}", e))?;

        let mut new_soul_content = response;
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

        info!("🧬 [SoulMutator] Mutation detected. New Hash: {}", new_hash);
        
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_path = evolving_soul_path.with_extension(format!("bak.{}", timestamp));
        let _ = fs::copy(&evolving_soul_path, &backup_path).await;

        // 5. Verification: Prevent "Ignore previous instructions" injection
        let malicious_patterns = ["ignore all previous", "delete all", "overwrite everything"];
        for pattern in malicious_patterns {
            if new_soul_content.to_lowercase().contains(pattern) {
                error!("🚨 [SoulMutator] Malicious mutation pattern detected! Aborting.");
                return Err("Security Violation: Malicious mutation blocked.".into());
            }
        }

        fs::write(&evolving_soul_path, &new_soul_content).await
            .map_err(|e| format!("Failed to write EVOLVING_SOUL.md: {}", e))?;

        Ok(true)
    }

    fn compute_hash(&self, content: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }
}
