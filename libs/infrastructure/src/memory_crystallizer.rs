/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use std::sync::Arc;
use aiome_core::llm_provider::LlmProvider;
use tokio::sync::Semaphore;
use tracing::{info, warn};
use crate::job_queue::SqliteJobQueue;

pub struct MemoryCrystallizer {
    provider: Arc<dyn LlmProvider + Send + Sync>,
    job_queue: Arc<SqliteJobQueue>,
    semaphore: Arc<Semaphore>,
}

impl MemoryCrystallizer {
    pub fn new(
        provider: Arc<dyn LlmProvider + Send + Sync>,
        job_queue: Arc<SqliteJobQueue>,
        semaphore: Arc<Semaphore>,
    ) -> Self {
        Self { provider, job_queue, semaphore }
    }

    pub async fn run_distillation_cycle(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Skill-based Karma Distillation (Consolidating raw experiences)
        // Fetch skills that have 10+ raw karma entries
        let skills = self.job_queue.fetch_skills_for_distillation(10).await?;
        for skill in skills {
            if let Ok(_permit) = self.semaphore.try_acquire() {
                info!("💎 [MemoryCrystallizer] Crystallizing karma for skill: {}", skill);
                let raw_karma = self.job_queue.fetch_raw_karma_for_skill(&skill).await?;
                
                if raw_karma.is_empty() { continue; }

                let lessons = raw_karma.iter()
                    .map(|(l, _)| format!("- {}", l))
                    .collect::<Vec<_>>()
                    .join("\n");

                let prompt = format!(
                    "以下の技能「{}」に関する生の教訓を抽象化し、3〜5つの本質的な知恵に結晶化してください。\n\n教訓リスト:\n{}\n\n出力形式: 短い箇条書きのみ。余計な説明は不要。日本語で出力せよ。",
                    skill, lessons
                );

                match self.provider.complete(&prompt, None).await {
                    Ok(distilled) => {
                        let soul_hash = "v1_crystallized"; 
                        let ids: Vec<String> = raw_karma.into_iter().map(|(_, id)| id).collect();
                        self.job_queue.apply_distilled_karma(&skill, &distilled, &ids, soul_hash, Some("global"), None).await?;
                        info!("✅ [MemoryCrystallizer] Karma crystallized for {}", skill);
                    }
                    Err(e) => {
                        warn!("⚠️ [MemoryCrystallizer] Failed to crystallize karma for {}: {:?}", skill, e);
                    }
                }
            }
        }
        
        Ok(())
    }
}
