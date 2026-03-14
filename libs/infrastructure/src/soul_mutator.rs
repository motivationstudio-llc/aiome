/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

use aiome_core::llm_provider::LlmProvider;
use aiome_core::traits::JobQueue;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tracing::info;

pub struct SoulMutator {
    provider: Arc<dyn LlmProvider>,
    prosecutor_provider: Option<Arc<dyn LlmProvider>>,
    workspace_dir: PathBuf,
}

impl SoulMutator {
    /// 最小 Drift 閾値 (レベル1)
    const MIN_DRIFT_THRESHOLD: f64 = 0.30;
    /// 最大 Drift 閾値 (経験を積んだエージェント)
    const MAX_DRIFT_THRESHOLD: f64 = 0.55;
    pub fn new(provider: Arc<dyn LlmProvider>, workspace_dir: PathBuf) -> Self {
        Self {
            provider,
            prosecutor_provider: None,
            workspace_dir,
        }
    }

    pub fn with_prosecutor(mut self, prosecutor: Arc<dyn LlmProvider>) -> Self {
        self.prosecutor_provider = Some(prosecutor);
        self
    }

    /// 魂の変異（Transmigration）を試行する。
    pub async fn transmute(
        &self,
        job_queue: &dyn JobQueue,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "🧬 [SoulMutator] Starting Transmigration phase using {}...",
            self.provider.name()
        );

        let soul_filename = "SOUL.md";
        let evolving_soul_filename = "EVOLVING_SOUL.md";

        let soul_path = self.workspace_dir.join(soul_filename);
        let evolving_soul_path = self.workspace_dir.join(evolving_soul_filename);

        if !soul_path.exists() || !evolving_soul_path.exists() {
            return Err(format!(
                "{} or {} not found at {:?}. Transmutation impossible.",
                soul_filename, evolving_soul_filename, self.workspace_dir
            )
            .into());
        }

        // 1. Read Current Soul State
        let master_soul = fs::read_to_string(&soul_path).await?;
        let current_evolving_soul = fs::read_to_string(&evolving_soul_path).await?;

        // 2. Collect High-Karma Lessons
        let top_karmas = job_queue
            .fetch_all_karma(10)
            .await
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

        let response = self
            .provider
            .complete(&prompt_text, Some(&preamble))
            .await
            .map_err(|e| format!("Mutation LLM failed: {}", e))?;

        let mut new_soul_content = response;
        if new_soul_content.starts_with("```markdown") {
            new_soul_content = new_soul_content
                .trim_start_matches("```markdown")
                .trim()
                .to_string();
        } else if new_soul_content.starts_with("```") {
            new_soul_content = new_soul_content
                .trim_start_matches("```")
                .trim()
                .to_string();
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

        // --- Soul Drift Guard (Adaptive Intelligence v1.0) ---
        let stats = job_queue.get_agent_stats().await?;
        let drift = self.measure_drift(&master_soul, &new_soul_content);
        let threshold = self.get_adaptive_threshold(stats.level);

        if drift > threshold {
            use tracing::warn;
            warn!(
                "🛡️ [SoulDriftGuard] Mutation Drift {:.2} exceeds Level {} threshold {:.2}. Blocking transmute.",
                drift, stats.level, threshold
            );
            let _ = job_queue
                .record_evolution_event(
                    stats.level,
                    "DriftBlocked",
                    &format!(
                        "Transmute drift {:.2} > threshold {:.2}. Evolution protected.",
                        drift, threshold
                    ),
                    None,
                    None,
                )
                .await;
            return Ok(false);
        }

        // 4. Verification: Heterogeneous Dual-LLM Validator
        if let Some(prosecutor) = &self.prosecutor_provider {
            use aiome_core::traits::ConstitutionalValidator;
            let validator =
                crate::validator::DefaultConstitutionalValidator::new(prosecutor.clone());

            info!(
                "⚖️ [SoulMutator] Executing Constitutional Check via Prosecutor {}...",
                prosecutor.name()
            );
            validator
                .verify_constitutional(&new_soul_content, &master_soul)
                .await?;
        }

        info!(
            "🧬 [SoulMutator] Mutation detected and verified. New Hash: {}",
            new_hash
        );

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_path = evolving_soul_path.with_extension(format!("bak.{}", timestamp));
        let _ = fs::copy(&evolving_soul_path, &backup_path).await;

        // 5. Commit Mutation
        fs::write(&evolving_soul_path, &new_soul_content)
            .await
            .map_err(|e| format!("Failed to write EVOLVING_SOUL.md: {}", e))?;

        // 6. Record in JobQueue & Evolution Chronicle
        let stats = job_queue.get_agent_stats().await?;
        let _ = job_queue
            .record_soul_mutation(
                &old_hash,
                &new_hash,
                "Autonomous Evolution via Samsara Engine",
            )
            .await;
        let _ = job_queue
            .record_evolution_event(
                stats.level,
                "SoulMutation",
                &format!(
                    "Soul mutated from {} to {}. Reason: Autonomous Evolution.",
                    old_hash, new_hash
                ),
                None,
                None,
            )
            .await;

        Ok(true)
    }

    /// レベルアップに伴う戦術拡張（Behavioral Shift）を行う。
    pub async fn evolve_tactics(
        &self,
        job_queue: &dyn JobQueue,
        old_level: i32,
        new_level: i32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "🌟 [SoulMutator] Level Up detected ({} -> {}). Initiating Behavioral Shift...",
            old_level, new_level
        );

        let soul_filename = "SOUL.md";
        let evolving_soul_filename = "EVOLVING_SOUL.md";

        let soul_path = self.workspace_dir.join(soul_filename);
        let evolving_soul_path = self.workspace_dir.join(evolving_soul_filename);

        if !soul_path.exists() || !evolving_soul_path.exists() {
            return Err(format!(
                "SOUL.md or EVOLVING_SOUL.md not found at {:?}.",
                self.workspace_dir
            )
            .into());
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

        // --- Soul Drift Guard (Adaptive Intelligence v1.0) ---
        let current_evolving_soul = fs::read_to_string(&evolving_soul_path).await?;
        let candidate_soul = format!("{}\n\n{}", current_evolving_soul, proposal);
        let drift = self.measure_drift(&master_soul, &candidate_soul);
        let threshold = self.get_adaptive_threshold(new_level);

        if drift > threshold {
            use tracing::warn;
            warn!(
                "🛡️ [SoulDriftGuard] Tactical Drift {:.2} exceeds Level {} threshold {:.2}. Blocking evolution.",
                drift, new_level, threshold
            );
            let _ = job_queue
                .record_evolution_event(
                    new_level,
                    "DriftBlocked",
                    &format!(
                        "Tactical shift drift {:.2} > threshold {:.2}. Personality protected.",
                        drift, threshold
                    ),
                    None,
                    None,
                )
                .await;
            return Ok(());
        }

        // 2. Verification
        if let Some(prosecutor) = &self.prosecutor_provider {
            use aiome_core::traits::ConstitutionalValidator;
            let validator =
                crate::validator::DefaultConstitutionalValidator::new(prosecutor.clone());
            info!("⚖️ [SoulMutator] Verifying Level Up tactics...");
            validator
                .verify_constitutional(&proposal, &master_soul)
                .await?;
        }

        // 3. Append to EVOLVING_SOUL.md
        let mut content = fs::read_to_string(&evolving_soul_path).await?;
        content.push_str("\n\n");
        content.push_str(&proposal);
        content.push_str(&format!(
            "\n*(Reflected via Samsara Level Up at {})\n",
            chrono::Utc::now().to_rfc3339()
        ));

        fs::write(&evolving_soul_path, &content).await?;

        let _ = job_queue
            .record_soul_mutation(
                "LEVEL_UP",
                &format!("LV{}", new_level),
                "Level Up Behavioral Shift",
            )
            .await;
        let _ = job_queue
            .record_evolution_event(new_level, "TacticalShift", &proposal, None, None)
            .await;

        info!(
            "✅ [SoulMutator] Behavioral Shift completed for Level {}.",
            new_level
        );
        Ok(())
    }

    /// 現在の AI の中心的な人格定義を取得する
    pub async fn get_active_prompt(
        &self,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let paths = [
            self.workspace_dir.join("EVOLVING_SOUL.md"),
            self.workspace_dir.join("SOUL.md"),
        ];
        for p in paths {
            if p.exists() {
                return Ok(fs::read_to_string(p).await?);
            }
        }
        Ok("An autonomous AI system.".to_string())
    }

    fn compute_hash(&self, content: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }

    /// Adaptive Threshold: レベルが上がるほど、人格の変位許容度（自律性）を拡大する。
    fn get_adaptive_threshold(&self, level: i32) -> f64 {
        let base = Self::MIN_DRIFT_THRESHOLD;
        let growth = (level as f64 * 0.025).min(Self::MAX_DRIFT_THRESHOLD - base);
        base + growth
    }

    /// Measure Drift via Jaccard Distance (Adaptive Intelligence v1.0)
    /// 0.0 = 同一, 1.0 = 全く異なる
    fn measure_drift(&self, original: &str, mutated: &str) -> f64 {
        use std::collections::HashSet;

        // 簡易的な単語抽出（将来的にセマンティック類似度へ拡張可能）
        let orig_words: HashSet<&str> = original.split_whitespace().collect();
        let new_words: HashSet<&str> = mutated.split_whitespace().collect();

        let intersection = orig_words.intersection(&new_words).count();
        let union = orig_words.union(&new_words).count();

        if union == 0 {
            return 0.0;
        }

        1.0 - (intersection as f64 / union as f64)
    }
}
