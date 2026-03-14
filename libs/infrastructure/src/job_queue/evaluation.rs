/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

use super::try_get_optional_string;
use super::SqliteJobQueue;
use aiome_core::contracts::OracleVerdict;
use aiome_core::error::AiomeError;
use aiome_core::traits::{Job, JobStatus, SnsMetricsRecord};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;

#[async_trait]
pub trait EvaluationOps {
    async fn do_fetch_jobs_for_evaluation(
        &self,
        milestone_days: i64,
        limit: i64,
    ) -> Result<Vec<Job>, AiomeError>;
    async fn do_record_sns_metrics(
        &self,
        job_id: &str,
        milestone_days: i64,
        views: i64,
        likes: i64,
        comments_count: i64,
        raw_comments: Option<&str>,
    ) -> Result<(), AiomeError>;
    async fn do_fetch_pending_evaluations(
        &self,
        limit: i64,
    ) -> Result<Vec<SnsMetricsRecord>, AiomeError>;
    async fn do_apply_final_verdict(
        &self,
        record_id: i64,
        verdict: OracleVerdict,
        soul_hash: &str,
    ) -> Result<(), AiomeError>;
    async fn do_fetch_top_performing_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError>;
}

#[async_trait]
impl EvaluationOps for SqliteJobQueue {
    async fn do_fetch_jobs_for_evaluation(
        &self,
        milestone_days: i64,
        limit: i64,
    ) -> Result<Vec<Job>, AiomeError> {
        let rows = sqlx::query(
            "SELECT id, category, topic, style_name, karma_directives, status, started_at, last_heartbeat, 
                     tech_karma_extracted, creative_rating, execution_log, error_message,
                     sns_platform, sns_content_id, published_at, output_artifacts 
              FROM jobs 
              WHERE sns_platform IS NOT NULL 
              AND sns_content_id IS NOT NULL 
              AND published_at IS NOT NULL
              AND published_at <= datetime('now', ? || ' days')
              AND id NOT IN (SELECT job_id FROM sns_metrics_history WHERE milestone_days = ?)
              ORDER BY published_at ASC LIMIT ?"
        )
        .bind(format!("-{}", milestone_days))
        .bind(milestone_days)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to fetch jobs for evaluation: {}", e) })?;

        let mut jobs = Vec::new();
        for r in rows {
            let tech_karma_extracted: i32 = r.get("tech_karma_extracted");
            jobs.push(Job {
                id: r.get("id"),
                category: r.get("category"),
                topic: r.get("topic"),
                style: r.get("style_name"),
                karma_directives: try_get_optional_string(&r, "karma_directives"),
                status: JobStatus::from_string(r.get("status")),
                started_at: try_get_optional_string(&r, "started_at"),
                last_heartbeat: try_get_optional_string(&r, "last_heartbeat"),
                tech_karma_extracted: tech_karma_extracted != 0,
                creative_rating: r.try_get("creative_rating").ok(),
                execution_log: try_get_optional_string(&r, "execution_log"),
                error_message: try_get_optional_string(&r, "error_message"),
                sns_platform: try_get_optional_string(&r, "sns_platform"),
                sns_content_id: try_get_optional_string(&r, "sns_content_id"),
                published_at: try_get_optional_string(&r, "published_at"),
                output_artifacts: try_get_optional_string(&r, "output_artifacts"),
            });
        }
        Ok(jobs)
    }

    async fn do_record_sns_metrics(
        &self,
        job_id: &str,
        milestone_days: i64,
        views: i64,
        likes: i64,
        comments_count: i64,
        raw_comments: Option<&str>,
    ) -> Result<(), AiomeError> {
        let now = Utc::now().to_rfc3339();

        let engagement_rate = if views > 0 {
            (likes as f64 / views as f64) * 100.0
        } else {
            0.0
        };

        let hard_metric_score = if engagement_rate >= 10.0 {
            1.0
        } else if engagement_rate >= 5.0 {
            0.5
        } else if engagement_rate >= 1.0 {
            0.0
        } else {
            -0.5
        };

        sqlx::query(
            "INSERT INTO sns_metrics_history (job_id, milestone_days, views, likes, comments_count, raw_comments_json, hard_metric_score, engagement_rate, recorded_at) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(job_id)
        .bind(milestone_days)
        .bind(views)
        .bind(likes)
        .bind(comments_count)
        .bind(raw_comments)
        .bind(hard_metric_score)
        .bind(engagement_rate)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to record SNS metrics: {}", e) })?;
        Ok(())
    }

    async fn do_fetch_pending_evaluations(
        &self,
        limit: i64,
    ) -> Result<Vec<SnsMetricsRecord>, AiomeError> {
        let rows = sqlx::query(
            "SELECT id, job_id, milestone_days, views, likes, comments_count, raw_comments_json, hard_metric_score, engagement_rate
             FROM sns_metrics_history 
             WHERE is_finalized = 0 
             ORDER BY recorded_at ASC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to fetch pending evaluations: {}", e) })?;

        let mut records = Vec::new();
        for r in rows {
            records.push(SnsMetricsRecord {
                id: r.get("id"),
                job_id: r.get("job_id"),
                milestone_days: r.get("milestone_days"),
                views: r.get("views"),
                likes: r.get("likes"),
                comments_count: r.get("comments_count"),
                raw_comments_json: try_get_optional_string(&r, "raw_comments_json"),
                hard_metric_score: r.try_get("hard_metric_score").ok(),
                engagement_rate: r.try_get("engagement_rate").ok(),
            });
        }
        Ok(records)
    }

    async fn do_apply_final_verdict(
        &self,
        record_id: i64,
        verdict: OracleVerdict,
        soul_hash: &str,
    ) -> Result<(), AiomeError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to start transaction: {}", e),
            })?;

        // 1. Update the Metrics Ledger (The Proof)
        sqlx::query(
            "UPDATE sns_metrics_history SET 
             alignment_score = ?, growth_score = ?, lesson = ?, should_evolve = ?, oracle_reason = ?, is_finalized = 1 
             WHERE id = ?"
        )
        .bind(verdict.alignment_score)
        .bind(verdict.growth_score)
        .bind(&verdict.lesson)
        .bind(verdict.should_evolve as i32)
        .bind(&verdict.reasoning)
        .bind(record_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to update ledger: {}", e) })?;

        // 2. Fetch job info for Karma update
        let job_row = sqlx::query(
            "SELECT j.id, j.topic, j.style_name, h.milestone_days 
             FROM jobs j 
             JOIN sns_metrics_history h ON j.id = h.job_id 
             WHERE h.id = ?",
        )
        .bind(record_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to fetch job context: {}", e),
        })?;

        let job_id: String = job_row.get("id");
        let style_name: String = job_row.get("style_name");
        let milestone_days: i64 = job_row.get("milestone_days");

        // 3. If it's the Final Verdict (30d), store the lesson in Karma Logs
        if milestone_days == 30 {
            let avg_score = (verdict.alignment_score + verdict.growth_score) / 2.0;
            let weight = (avg_score * 100.0) as i64;
            let weight = weight.clamp(0, 100);

            let karma_id = Uuid::new_v4().to_string();
            let now = Utc::now().to_rfc3339();

            let (domain, subtopic) = match &verdict.classification {
                Some(c) => (Some(c.domain.as_str()), Some(c.subtopic.as_str())),
                None => (None, None),
            };

            sqlx::query(
                "INSERT INTO karma_logs (id, job_id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, domain, subtopic)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&karma_id)
            .bind(&job_id)
            .bind("Synthesized")
            .bind(&style_name)
            .bind(&verdict.lesson)
            .bind(weight)
            .bind(soul_hash)
            .bind(&now)
            .bind(domain.unwrap_or("general"))
            .bind(subtopic)
            .execute(&mut *tx)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to update Karma logs: {}", e) })?;
        }

        // 4. Update Agent Stats (Growth - optional heuristic)
        if verdict.should_evolve {
            sqlx::query("UPDATE agent_stats SET exp = exp + 10, resonance = resonance + 5, updated_at = datetime('now') WHERE id = 1")
                .execute(&mut *tx).await.ok();
        }

        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to commit transaction: {}", e),
        })?;
        Ok(())
    }

    async fn do_fetch_top_performing_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError> {
        let rows = sqlx::query(
            "SELECT j.* FROM jobs j 
             JOIN sns_metrics_history s ON j.id = s.job_id 
             WHERE s.is_finalized = 1 
             ORDER BY s.views DESC 
             LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: e.to_string(),
        })?;

        let mut jobs = Vec::new();
        for r in rows {
            let tech_karma_extracted: i32 = r.get("tech_karma_extracted");
            jobs.push(Job {
                id: r.get("id"),
                category: r.get("category"),
                topic: r.get("topic"),
                style: r.get("style_name"),
                karma_directives: try_get_optional_string(&r, "karma_directives"),
                status: JobStatus::from_string(r.get::<String, _>("status").as_str()),
                started_at: try_get_optional_string(&r, "started_at"),
                last_heartbeat: try_get_optional_string(&r, "last_heartbeat"),
                tech_karma_extracted: tech_karma_extracted != 0,
                creative_rating: r.try_get("creative_rating").ok(),
                execution_log: try_get_optional_string(&r, "execution_log"),
                error_message: try_get_optional_string(&r, "error_message"),
                sns_platform: try_get_optional_string(&r, "sns_platform"),
                sns_content_id: try_get_optional_string(&r, "sns_content_id"),
                published_at: try_get_optional_string(&r, "published_at"),
                output_artifacts: try_get_optional_string(&r, "output_artifacts"),
            });
        }
        Ok(jobs)
    }
}
