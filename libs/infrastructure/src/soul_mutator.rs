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
use tracing::info;

pub struct SoulMutator {
    provider: Arc<dyn LlmProvider>,
    prosecutor_provider: Option<Arc<dyn LlmProvider>>,
}

impl SoulMutator {
    pub fn new(provider: Arc<dyn LlmProvider>, _workspace_dir: PathBuf) -> Self {
        Self { provider, prosecutor_provider: None }
    }

    pub fn with_prosecutor(mut self, prosecutor: Arc<dyn LlmProvider>) -> Self {
        self.prosecutor_provider = Some(prosecutor);
        self
    }

    /// 魂の変異（Transmigration）を試行する。
    pub async fn transmute(&self, job_queue: &dyn JobQueue) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        info!("🧬 [SoulMutator] Starting Transmigration phase using {}...", self.provider.name());

        let soul_filename = "SOUL.md";
        let evolving_soul_filename = "EVOLVING_SOUL.md";
        
        let mut soul_path = std::path::PathBuf::from(soul_filename);
        let mut evolving_soul_path = std::path::PathBuf::from(evolving_soul_filename);

        if !soul_path.exists() {
            let parent_soul = std::path::PathBuf::from(format!("../../{}", soul_filename));
            if parent_soul.exists() {
                soul_path = parent_soul;
                evolving_soul_path = std::path::PathBuf::from(format!("../../{}", evolving_soul_filename));
            }
        }

        if !soul_path.exists() || !evolving_soul_path.exists() {
            return Err(format!("{} or {} not found. Transmutation impossible.", soul_filename, evolving_soul_filename).into());
        }

        // 1. Read Current Soul State
        let master_soul = fs::read_to_string(&soul_path).await?;
        let current_evolving_soul = fs::read_to_string(&evolving_soul_path).await?;

        // 2. Collect High-Karma Lessons
        let top_karmas = job_queue.fetch_all_karma(10).await
            .map_err(|e| format!("Failed to fetch karma: {}", e))?;
        
        // Filter Technical/Creative high-weight karma manually as a proxy for fetch_relevant_karma
        let mut lessons = Vec::new();
        for k in top_karmas {
            if let Some(lesson) = k["lesson"].as_str() {
                lessons.push(format!("- {}", lesson));
            }
        }

        if lessons.is_empty() {
             info!("🧬 [SoulMutator] Not enough high-quality Karma accumulated yet. Skipping mutation.");
             return Ok(false);
        }

        // 3. Mutation Generation
        let preamble = format!(
            "AI の魂の進化プロセスの継続。EVOLVING_SOUL.md を更新せよ。\n\nユーザーソウル (核):\n{}\n\n蓄積された教訓:\n{}",
            master_soul,
            lessons.join("\n")
        );

        let prompt_text = format!("現在のあなたの進化状況を反映した、最新の EVOLVING_SOUL.md を生成せよ。現状を否定せず、教訓を取り入れて拡張すること。\n\n現在の内容:\n{}", current_evolving_soul);

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

        // 4. Verification: Heterogeneous Dual-LLM Validator
        if let Some(prosecutor) = &self.prosecutor_provider {
            use aiome_core::traits::ConstitutionalValidator;
            let validator = crate::validator::DefaultConstitutionalValidator::new(prosecutor.clone());
            
            info!("⚖️ [SoulMutator] Executing Constitutional Check via Prosecutor {}...", prosecutor.name());
            validator.verify_constitutional(&new_soul_content, &master_soul).await?;
        }

        info!("🧬 [SoulMutator] Mutation detected and verified. New Hash: {}", new_hash);
        
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_path = evolving_soul_path.with_extension(format!("bak.{}", timestamp));
        let _ = fs::copy(&evolving_soul_path, &backup_path).await;

        // 5. Commit Mutation
        fs::write(&evolving_soul_path, &new_soul_content).await
            .map_err(|e| format!("Failed to write EVOLVING_SOUL.md: {}", e))?;

        // 6. Record in JobQueue & Evolution Chronicle
        let stats = job_queue.get_agent_stats().await?;
        let _ = job_queue.record_soul_mutation(&old_hash, &new_hash, "Autonomous Evolution via Samsara Engine").await;
        let _ = job_queue.record_evolution_event(stats.level, "SoulMutation", &format!("Soul mutated from {} to {}. Reason: Autonomous Evolution.", old_hash, new_hash), None, None).await;

        Ok(true)
    }

    /// レベルアップに伴う戦術拡張（Behavioral Shift）を行う。
    pub async fn evolve_tactics(&self, job_queue: &dyn JobQueue, old_level: i32, new_level: i32) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("🌟 [SoulMutator] Level Up detected ({} -> {}). Initiating Behavioral Shift...", old_level, new_level);

        let soul_filename = "SOUL.md";
        let evolving_soul_filename = "EVOLVING_SOUL.md";
        
        let mut soul_path = std::path::PathBuf::from(soul_filename);
        let mut evolving_soul_path = std::path::PathBuf::from(evolving_soul_filename);

        if !soul_path.exists() {
            let parent_soul = std::path::PathBuf::from(format!("../../{}", soul_filename));
            if parent_soul.exists() {
                soul_path = parent_soul;
                evolving_soul_path = std::path::PathBuf::from(format!("../../{}", evolving_soul_filename));
            }
        }

        if !soul_path.exists() || !evolving_soul_path.exists() {
            return Err("SOUL.md or EVOLVING_SOUL.md not found.".into());
        }

        let master_soul = fs::read_to_string(&soul_path).await?;
        
        // 1. Generate New Tactics
        let preamble = format!(
            "あなたはAiome OSの進化エンジンです。レベルアップに伴う行動変容(Behavioral Shift)を計画してください。\n\n現在のレベル: {}\n新しいレベル: {}\n\n憲法 (核):\n{}",
            old_level,
            new_level,
            master_soul
        );

        let prompt = format!(
            "レベルが {} に到達しました。現在の能力を最大限に活かし、より自律的、かつ協調的な行動をとるための「新しい行動方針」を 1つ提案してください。\n\
            出力フォーマット:\n### Level {} Shift: [方針名]\n[具体的な方針内容 (2-3文)]",
            new_level,
            new_level
        );

        let proposal = self.provider.complete(&prompt, Some(&preamble)).await?;

        // 2. Verification
        if let Some(prosecutor) = &self.prosecutor_provider {
            use aiome_core::traits::ConstitutionalValidator;
            let validator = crate::validator::DefaultConstitutionalValidator::new(prosecutor.clone());
            info!("⚖️ [SoulMutator] Verifying Level Up tactics...");
            validator.verify_constitutional(&proposal, &master_soul).await?;
        }

        // 3. Append to EVOLVING_SOUL.md
        let mut content = fs::read_to_string(&evolving_soul_path).await?;
        content.push_str("\n\n");
        content.push_str(&proposal);
        content.push_str(&format!("\n*(Reflected via Samsara Level Up at {})\n", chrono::Utc::now().to_rfc3339()));

        fs::write(&evolving_soul_path, &content).await?;
        
        let _ = job_queue.record_soul_mutation("LEVEL_UP", &format!("LV{}", new_level), "Level Up Behavioral Shift").await;
        let _ = job_queue.record_evolution_event(new_level, "TacticalShift", &proposal, None, None).await;

        info!("✅ [SoulMutator] Behavioral Shift completed for Level {}.", new_level);
        Ok(())
    }

    fn compute_hash(&self, content: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }
}
