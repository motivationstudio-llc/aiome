/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 * You may not provide the software to third parties as a hosted or managed service, 
 * where the service provides users with access to any substantial set of the features 
 * or functionality of the software.
 */

use aiome_core::traits::{JobQueue, TrendSource, JobStatus};
use crate::trend_sonar::ExternalTrendSonar;
use tracing::{info, warn};
use std::error::Error;

pub struct DreamState {}

impl DreamState {
    pub fn new() -> Self {
        Self {}
    }

    /// 「夢想状態（Dream State）」を実行する。
    /// キューが空の時に、自発的なトレンド探索や過去の失敗への内省を行う。
    pub async fn dream(&self, job_queue: &dyn JobQueue, trend_sonar: &ExternalTrendSonar) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("💤 [DreamState] AI is entering a contemplative Dream State...");

        // 1. Preemption Check: キューに仕事があるなら即座に起きる
        let pending = job_queue.get_pending_job_count().await?;
        if pending > 0 {
            info!("💤 [DreamState] Real tasks detected. Terminating dream and waking up.");
            return Ok(());
        }

        // 2. Decide Dream Type
        let now = chrono::Utc::now().timestamp();
        if now % 2 == 0 {
            self.explorative_dream(job_queue, trend_sonar).await?;
        } else {
            self.reflective_dream(job_queue).await?;
        }

        Ok(())
    }

    /// 探索夢: TrendSonarを使って面白いトピックを拾い、将来のジョブとして予約する
    async fn explorative_dream(&self, job_queue: &dyn JobQueue, trend_sonar: &ExternalTrendSonar) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("💤 [DreamState] Mode: Explorative — Searching for new creative horizons...");
        
        let seeds = ["cyberpunk aesthetics", "ancient lost technology", "biomimicry", "lo-fi horror", "solarpunk architecture"];
        let seed = seeds[(chrono::Utc::now().timestamp() as usize) % seeds.len()];
        
        match trend_sonar.get_trends(seed).await {
            Ok(trends) if !trends.is_empty() => {
                // 最もスコアの高いものを「幻（Phantom）」ジョブとして投入
                let best = &trends[0];
                info!("🔮 [DreamState] Dreamt of a new possibility: '{}'. Seeded into the cycle.", best.keyword);
                
                // phantomフラグ付きで投入（Orchestrator側で、誰もいない時に優先的に拾われる等の処理が可能）
                let directives = format!("{{\"dream_born\": true, \"seed\": \"{}\", \"phantom\": true}}", seed);
                job_queue.enqueue("data_processing", &best.keyword, "auto", Some(&directives)).await?;
            }
            Ok(_) => warn!("💤 [DreamState] The dream was a void. No trends found."),
            Err(e) => warn!("💤 [DreamState] Dream vision blurred: {}", e),
        }
        
        Ok(())
    }

    /// 省察夢: 過去の失敗を振り返り、Karmaの重要度を再評価する（または再試行を検討する）
    async fn reflective_dream(&self, job_queue: &dyn JobQueue) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("💤 [DreamState] Mode: Reflective — Contemplating past scars and lessons...");
        
        let recent = job_queue.fetch_all_karma(10).await?;
        if recent.is_empty() {
             info!("💤 [DreamState] No memories to reflect upon yet.");
             return Ok(());
        }

        // 失敗したジョブを1つ選び、そのトピックを少し変えて再投入することを「夢」とする
        let recent_jobs = job_queue.fetch_recent_jobs(20).await?;
        let failed_jobs: Vec<_> = recent_jobs.iter()
            .filter(|j| matches!(j.status, JobStatus::Failed))
            .collect();

        if let Some(fail) = failed_jobs.first() {
            info!("🩹 [DreamState] Remembering the failure of '{}'. Dreaming of a redemption version...", fail.topic);
            let redemption_topic = format!("{} (Redemption Remix)", fail.topic);
            let directives = format!("{{\"remix_of\": \"{}\", \"dream_born\": true}}", fail.id);
            job_queue.enqueue("data_processing", &redemption_topic, &fail.style, Some(&directives)).await?;
        } else {
            info!("✨ [DreamState] The past is clear. No recent failures haunt my dreams.");
        }

        Ok(())
    }
}
