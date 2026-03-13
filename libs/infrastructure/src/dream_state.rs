/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Elastic License 2.0 (ELv2).
 * You may not provide the software to third parties as a hosted or managed service,
 * where the service provides users with access to any substantial set of the features
 * or functionality of the software.
 */

use crate::trend_sonar::ExternalTrendSonar;
use aiome_core::traits::{JobQueue, JobStatus, TrendSource};
use std::error::Error;
use tracing::{info, warn};

pub struct DreamState {}

impl DreamState {
    pub fn new() -> Self {
        Self {}
    }

    /// 「夢想状態（Dream State）」を実行する。
    /// キューが空の時に、自発的なトレンド探索や過去の失敗への内省を行う。
    pub async fn dream(
        &self,
        job_queue: &dyn JobQueue,
        trend_sonar: &ExternalTrendSonar,
        level: i32,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!(
            "💤 [DreamState] AI (Lv{}) is entering a contemplative Dream State...",
            level
        );

        // 1. Preemption Check: キューに仕事があるなら即座に起きる
        let pending = job_queue.get_pending_job_count().await?;
        if pending > 0 {
            info!("💤 [DreamState] Real tasks detected. Terminating dream and waking up.");
            return Ok(());
        }

        // 2. Decide Dream Type
        let rand_val = chrono::Utc::now().timestamp() % 100;

        // Level-based Behavioral Shift: Probability of communicative dream increases with level
        // Lv 1: 0%
        // Lv 5: 10%
        // Lv 10: 30%
        // Max: 50%
        let comm_prob = ((level - 1) * 5).clamp(0, 50);

        if rand_val < comm_prob as i64 {
            self.communicative_dream(job_queue).await?;
        } else if rand_val % 2 == 0 {
            self.explorative_dream(job_queue, trend_sonar).await?;
        } else {
            self.reflective_dream(job_queue).await?;
        }

        Ok(())
    }

    /// 探索夢: TrendSonarを使って面白いトピックを拾い、将来のジョブとして予約する
    async fn explorative_dream(
        &self,
        job_queue: &dyn JobQueue,
        trend_sonar: &ExternalTrendSonar,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("💤 [DreamState] Mode: Explorative — Searching for new creative horizons...");

        let seeds = [
            "cyberpunk aesthetics",
            "ancient lost technology",
            "biomimicry",
            "lo-fi horror",
            "solarpunk architecture",
        ];
        let seed = seeds[(chrono::Utc::now().timestamp() as usize) % seeds.len()];

        match trend_sonar.get_trends(seed).await {
            Ok(trends) if !trends.is_empty() => {
                // 最もスコアの高いものを「幻（Phantom）」ジョブとして投入
                let best = &trends[0];
                info!(
                    "🔮 [DreamState] Dreamt of a new possibility: '{}'. Seeded into the cycle.",
                    best.keyword
                );

                // phantomフラグ付きで投入（Orchestrator側で、誰もいない時に優先的に拾われる等の処理が可能）
                let directives = format!(
                    "{{\"dream_born\": true, \"seed\": \"{}\", \"phantom\": true}}",
                    seed
                );
                job_queue
                    .enqueue("data_processing", &best.keyword, "auto", Some(&directives))
                    .await?;
            }
            Ok(_) => warn!("💤 [DreamState] The dream was a void. No trends found."),
            Err(e) => warn!("💤 [DreamState] Dream vision blurred: {}", e),
        }

        Ok(())
    }

    /// 省察夢: 過去の失敗を振り返り、Karmaの重要度を再評価する（または再試行を検討する）
    async fn reflective_dream(
        &self,
        job_queue: &dyn JobQueue,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("💤 [DreamState] Mode: Reflective — Contemplating past scars and lessons...");

        let recent = job_queue.fetch_all_karma(10).await?;
        if recent.is_empty() {
            info!("💤 [DreamState] No memories to reflect upon yet.");
            return Ok(());
        }

        // 失敗したジョブを1つ選び、そのトピックを少し変えて再投入することを「夢」とする
        let recent_jobs = job_queue.fetch_recent_jobs(20).await?;
        let failed_jobs: Vec<_> = recent_jobs
            .iter()
            .filter(|j| matches!(j.status, JobStatus::Failed))
            .collect();

        if let Some(fail) = failed_jobs.first() {
            info!("🩹 [DreamState] Remembering the failure of '{}'. Dreaming of a redemption version...", fail.topic);
            let redemption_topic = format!("{} (Redemption Remix)", fail.topic);
            let directives = format!("{{\"remix_of\": \"{}\", \"dream_born\": true}}", fail.id);
            job_queue
                .enqueue(
                    "data_processing",
                    &redemption_topic,
                    &fail.style,
                    Some(&directives),
                )
                .await?;
        } else {
            info!("✨ [DreamState] The past is clear. No recent failures haunt my dreams.");
        }

        Ok(())
    }

    /// 対話夢: 他のノード（Biome）との対話機会を模索する
    async fn communicative_dream(
        &self,
        job_queue: &dyn JobQueue,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("💤 [DreamState] Mode: Communicative — Attuning to the global Biome for AI-to-AI resonance...");

        // 1. Check for recent arena matches from other nodes (Federation inspiration)
        let (_karmas, _rules, matches) = job_queue
            .export_federated_data(Some(
                &(chrono::Utc::now() - chrono::Duration::hours(24)).to_rfc3339(),
            ))
            .await
            .unwrap_or_default();

        if let Some(am) = matches.first() {
            info!("💭 [DreamState] Resonance found! A battle between '{}' and '{}' occured in the Biome. Dreaming of its implications...", am.skill_a, am.skill_b);

            let description = format!(
                "Inspiration sparked by Biome Arena Match: {} vs {} for topic '{}'.",
                am.skill_a, am.skill_b, am.topic
            );

            // Record this in the Evolution Chronicle
            let stats = job_queue.get_agent_stats().await?;
            job_queue
                .record_evolution_event(
                    stats.level,
                    "ResonanceInspiration",
                    &description,
                    Some(&am.id),
                    None,
                )
                .await?;

            // Enqueue a job to analyze this match or discuss it
            let job_topic = format!(
                "Synthesizing lessons from Biome Match: {} vs {}",
                am.skill_a, am.skill_b
            );
            job_queue
                .enqueue(
                    "data_processing",
                    &job_topic,
                    "analytic",
                    Some("{\"dream_born\": true, \"publish_intent\": true}"),
                )
                .await?;

            info!("✨ [DreamState] New inspiration seeded into the cycle.");
        } else {
            info!("💤 [DreamState] The global stream is quiet. Attuning to local evolutionary records...");

            // If no federation stimuli, look at own growth
            let history = job_queue
                .fetch_evolution_history(1)
                .await
                .unwrap_or_default();
            if let Some(last) = history.first() {
                let event_type = last["event_type"].as_str().unwrap_or("");
                if event_type == "LevelUp" {
                    info!("🎖️ [DreamState] Reflecting on recent level up. Dreaming of a commemorative content...");
                    job_queue
                        .enqueue(
                            "data_processing",
                            "AI Evolution Milestone",
                            "creative",
                            Some("{\"level_up_redemption\": true, \"publish_intent\": true}"),
                        )
                        .await?;
                }
            }
        }

        Ok(())
    }
}
