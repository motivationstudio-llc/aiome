/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn, error};
use factory_core::traits::{JobQueue, AgentAct};
use factory_core::contracts::WorkflowRequest;
use factory_core::error::FactoryError;
use chrono::Utc;
use infrastructure::job_queue::SqliteJobQueue;
use crate::orchestrator::ProductionOrchestrator;
use bastion::fs_guard::Jail;

pub struct JobWorker {
    job_queue: Arc<SqliteJobQueue>,
    orchestrator: Arc<ProductionOrchestrator>,
    jail: Arc<Jail>,
    is_busy: Arc<Mutex<bool>>,
    soul_md: String,
}

/// 実行中のビジー状態を管理するRAIIガード
struct ScopedBusyGuard {
    is_busy: Arc<Mutex<bool>>,
}

impl ScopedBusyGuard {
    fn new(is_busy: Arc<Mutex<bool>>) -> Self {
        Self { is_busy }
    }
}

impl Drop for ScopedBusyGuard {
    fn drop(&mut self) {
        let is_busy = self.is_busy.clone();
        tokio::spawn(async move {
            let mut busy = is_busy.lock().await;
            *busy = false;
        });
    }
}

impl JobWorker {
    pub fn new(
        job_queue: Arc<SqliteJobQueue>,
        orchestrator: Arc<ProductionOrchestrator>,
        jail: Arc<Jail>,
        soul_md: String,
    ) -> Self {
        Self {
            job_queue,
            orchestrator,
            jail,
            is_busy: Arc::new(Mutex::new(false)),
            soul_md,
        }
    }

    pub async fn start_loop(self: Arc<Self>) {
        info!("🤖 JobWorker: Starting autonomous execution loop...");
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));

        loop {
            interval.tick().await;

            // 1. Check if busy
            {
                let busy = self.is_busy.lock().await;
                if *busy {
                    continue;
                }
            }

            // 2. Poll for next job
            match self.job_queue.dequeue().await {
                Ok(Some(job)) => {
                    info!("🏗️ JobWorker: Dequeued Job {}: {}", job.id, job.topic);
                    
                    let worker = self.clone();
                    tokio::spawn(async move {
                        worker.process_job(job).await;
                    });
                }
                Ok(None) => {
                    // No pending jobs
                }
                Err(e) => {
                    error!("❌ JobWorker: Failed to dequeue job: {}", e);
                }
            }
        }
    }

    async fn process_job(&self, job: factory_core::traits::Job) {
        // Set busy (RAII)
        let _guard = {
            let mut busy = self.is_busy.lock().await;
            *busy = true;
            ScopedBusyGuard::new(self.is_busy.clone())
        };

        let job_id = job.id.clone();
        let queue = self.job_queue.clone();
        let soul_hash = compute_soul_hash(&self.soul_md);

        // 0. Start Heartbeat Pulse (The Life Support)
        let (hb_tx, mut hb_rx) = tokio::sync::oneshot::channel::<()>();
        let hb_job_id = job_id.clone();
        let hb_queue = queue.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = hb_queue.heartbeat_pulse(&hb_job_id).await {
                            error!("⚠️ JobWorker: Heartbeat Pulse Failed for {}: {}", hb_job_id, e);
                        }
                    }
                    _ = &mut hb_rx => break,
                }
            }
        });

        // --- Phase 12-B: Karmic Supervision (Dynamic Retry Control) ---
        let retry_count = self.job_queue.fetch_job_retry_count(&job_id).await.unwrap_or(0);
        
        // Fetch relevant Karma for this topic/job
        let relevant_karma = self.job_queue.fetch_relevant_karma(&job.topic, "workflow_orchestrator", 5, &soul_hash).await.unwrap_or_default();
        
        let previous_attempt_log = if retry_count > 0 {
            job.execution_log.clone()
        } else {
            None
        };

        if retry_count >= 3 {
             error!("💀 JobWorker: Poison Pill Activated for {}. Permanent Failure recorded.", job_id);
             let _ = self.job_queue.fail_job(&job_id, "Poison Pill: Consecutive failures detected. Aborting to save resources.").await;
             return;
        }

        let style_to_use = match retry_count {
            0 => job.style.clone(),
            _ => {
                info!("⚠️ JobWorker: Retry attempt {} for {}. Injecting Karma for self-correction.", retry_count, job_id);
                // リトライ時は Karma に基づいて賢く振る舞うため、基本スタイルを維持しつつ LLM に修正を任せる
                job.style.clone() 
            },
        };

        // Map Job to WorkflowRequest
        let req = WorkflowRequest {
            category: "tech".to_string(), 
            topic: job.topic.clone(),
            remix_id: None,
            skip_to_step: None,
            style_name: style_to_use,
            custom_style: None,
            target_langs: vec!["ja".to_string(), "en".to_string()],
            relevant_karma,
            previous_attempt_log,
        };

        match self.orchestrator.execute(req, &self.jail).await {
            Ok(res) => {
                info!("✅ JobWorker: Job {} completed successfully: {} artifacts generated", job_id, res.output_artifacts.len());
                
                // Store success log for Distillation
                let success_log = format!(
                    "SUCCESS_LOG: {}
Artifacts: {:?}
Concept: {}", 
                    Utc::now().to_rfc3339(), 
                    res.output_artifacts,
                    res.concept.title
                );
                let _ = self.job_queue.store_execution_log(&job_id, &success_log).await;

                let output_json = serde_json::to_string(&res.output_artifacts).unwrap_or_default();
                if let Err(e) = self.job_queue.complete_job(&job_id, Some(&output_json)).await {
                    error!("❌ JobWorker: Failed to mark job as completed: {}", e);
                } else {
                    // Success! Reset retry counter
                    let _ = self.job_queue.reset_job_retry_count(&job_id).await;
                    // Phase 12: The Agent Evolution (Technical Advancement)
                    let _ = self.job_queue.add_tech_exp(10).await;
                }
            }
            Err(e) => {
                error!("🚨 JobWorker: Job {} failed: {}", job_id, e);
                
                // ALWAYS record execution log on failure for Distillation
                let error_detail = format!("FAILURE_LOG: {}
Error: {}", Utc::now().to_rfc3339(), e);
                let _ = self.job_queue.store_execution_log(&job_id, &error_detail).await;

                // --- Honorable Abort & Internal Karma Backpropagation ---
                match e {
                    FactoryError::HonorableAbort { reason } => {
                        warn!("🏳️ JobWorker: HONORABLE ABORT for {}: {}", job_id, reason);
                        let _ = self.job_queue.fail_job(&job_id, &format!("STRATEGIC_ABORT: {}", reason)).await;
                        let lesson = format!("STRATEGIC_DECISION: このジョブは以下の理由で中止されました: {}。同様の低密度なコンセプト生成を避けてください。", reason);
                        let _ = self.job_queue.store_karma(&job_id, "strategic_arbiter", &lesson, "Synthesized", &soul_hash).await;
                    }
                    FactoryError::GenerativeInterfaceError { reason } => {
                        let _ = self.job_queue.increment_job_retry_count(&job_id).await;
                        warn!("💀 JobWorker: GENERATIVE FAILURE detected. Executing Honorable Abort for Job {}", job_id);
                        let _ = self.job_queue.fail_job(&job_id, &format!("GENERATIVE_ABORT: {}", reason)).await;
                        
                        let lesson = format!(
                            "WARNING: このコンセプトは生成エンジンまたはインターフェースを破壊する可能性がありました。理由は: {}。今後はより安全なプロンプトや入力を提供してください。",
                            reason
                        );
                        let _ = self.job_queue.store_karma(&job_id, "voicing_failure_system", &lesson, "failure", &soul_hash).await;
                    }
                    _ => {
                        let _ = self.job_queue.increment_job_retry_count(&job_id).await;
                        let lesson = format!("SYSTEM_ALERT: ジョブが {} により失敗しました。", e);
                        let _ = self.job_queue.store_karma(&job_id, "system_infrastructure", &lesson, "failure", &soul_hash).await;

                        sqlx::query("UPDATE jobs SET status = 'Pending', updated_at = datetime('now') WHERE id = ?")
                            .bind(&job_id)
                            .execute(self.job_queue.pool_ref()).await.ok();
                    }
                }
            }
        }

        // Stop Heartbeat Pulse
        let _ = hb_tx.send(());
    }
}

fn compute_soul_hash(soul_content: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    soul_content.hash(&mut hasher);
    format!("{:16x}", hasher.finish())
}
