/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 */

use async_trait::async_trait;
use aiome_core::traits::{Job, JobStatus};
use aiome_core::error::AiomeError;
use sqlx::Row;
use uuid::Uuid;
use chrono::Utc;
use super::SqliteJobQueue;
use super::try_get_optional_string;

#[async_trait]
pub trait CoreOps {
    async fn do_enqueue(&self, category: &str, topic: &str, style: &str, karma_directives: Option<&str>) -> Result<String, AiomeError>;
    async fn do_fetch_job(&self, job_id: &str) -> Result<Option<Job>, AiomeError>;
    async fn do_dequeue(&self, capable_categories: &[&str]) -> Result<Option<Job>, AiomeError>;
    async fn do_complete_job(&self, job_id: &str, output_artifacts: Option<&str>) -> Result<(), AiomeError>;
    async fn do_fail_job(&self, job_id: &str, reason: &str) -> Result<(), AiomeError>;
    async fn do_reclaim_zombie_jobs(&self, timeout_minutes: i64) -> Result<u64, AiomeError>;
    async fn do_set_creative_rating(&self, job_id: &str, rating: i32) -> Result<(), AiomeError>;
    async fn do_heartbeat_pulse(&self, job_id: &str) -> Result<(), AiomeError>;
    async fn do_store_execution_log(&self, job_id: &str, log: &str) -> Result<(), AiomeError>;
    async fn do_purge_old_jobs(&self, days: i64) -> Result<u64, AiomeError>;
    async fn do_fetch_recent_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError>;
    async fn do_get_pending_job_count(&self) -> Result<i64, AiomeError>;
    async fn do_get_job_count_since(&self, since: chrono::DateTime<chrono::Utc>) -> Result<i64, AiomeError>;
    async fn do_fetch_job_retry_count(&self, job_id: &str) -> Result<i64, AiomeError>;
    async fn do_increment_job_retry_count(&self, job_id: &str) -> Result<bool, AiomeError>;
    async fn do_reset_job_retry_count(&self, job_id: &str) -> Result<(), AiomeError>;
    async fn do_storage_gc(&self, threshold_gb: f64) -> Result<u64, AiomeError>;
}

#[async_trait]
impl CoreOps for SqliteJobQueue {
    async fn do_enqueue(&self, category: &str, topic: &str, style: &str, karma_directives: Option<&str>) -> Result<String, AiomeError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let directives = karma_directives.unwrap_or("{}");

        sqlx::query(
            "INSERT INTO jobs (id, category, topic, style_name, karma_directives, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(category)
        .bind(topic)
        .bind(style)
        .bind(directives)
        .bind(JobStatus::Pending.to_string())
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to enqueue job: {}", e) })?;

        Ok(id)
    }

    async fn do_fetch_job(&self, job_id: &str) -> Result<Option<Job>, AiomeError> {
        let row = sqlx::query(
            "SELECT id, category, topic, style_name, karma_directives, status, started_at, last_heartbeat, tech_karma_extracted, creative_rating, execution_log, error_message, sns_platform, sns_content_id, published_at, output_artifacts FROM jobs WHERE id = ?"
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to fetch job {}: {}", job_id, e) })?;

        if let Some(r) = row {
            let id: String = r.get("id");
            let category: String = r.get("category");
            let topic: String = r.get("topic");
            let style: String = r.get("style_name");
            let karma_directives: Option<String> = try_get_optional_string(&r, "karma_directives");
            let tech_karma_extracted: i32 = r.get("tech_karma_extracted");
            let creative_rating: Option<i32> = r.try_get("creative_rating").ok();
            let execution_log: Option<String> = try_get_optional_string(&r, "execution_log");
            let error_message: Option<String> = try_get_optional_string(&r, "error_message");
            let sns_platform: Option<String> = try_get_optional_string(&r, "sns_platform");
            let sns_content_id: Option<String> = try_get_optional_string(&r, "sns_content_id");
            let published_at: Option<String> = try_get_optional_string(&r, "published_at");
            let output_artifacts: Option<String> = try_get_optional_string(&r, "output_artifacts");
            let status_str: String = r.get("status");
            let status = JobStatus::from_string(&status_str);

            Ok(Some(Job {
                id,
                category,
                topic,
                style,
                karma_directives,
                status,
                started_at: r.get("started_at"),
                last_heartbeat: r.get("last_heartbeat"),
                tech_karma_extracted: tech_karma_extracted != 0,
                creative_rating,
                execution_log,
                error_message,
                sns_platform,
                sns_content_id,
                published_at,
                output_artifacts,
            }))
        } else {
            Ok(None)
        }
    }

    async fn do_dequeue(&self, capable_categories: &[&str]) -> Result<Option<Job>, AiomeError> {
        let mut tx = self.pool.begin().await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to start transaction: {}", e) })?;

        let placeholders = capable_categories.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let query_str = format!(
            "SELECT id, category, topic, style_name, karma_directives, status, started_at, last_heartbeat, tech_karma_extracted, creative_rating, execution_log, error_message, sns_platform, sns_content_id, published_at, output_artifacts FROM jobs WHERE status = ? AND category IN ({}) ORDER BY created_at ASC LIMIT 1",
            placeholders
        );
        let mut query = sqlx::query(&query_str).bind(JobStatus::Pending.to_string());
        for cat in capable_categories {
            query = query.bind(*cat);
        }

        let row = query.fetch_optional(&mut *tx)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to fetch pending job: {}", e) })?;

        if let Some(r) = row {
            let id: String = r.get("id");
            let topic: String = r.get("topic");
            let style: String = r.get("style_name");
            let karma_directives: Option<String> = try_get_optional_string(&r, "karma_directives");
            let tech_karma_extracted: i32 = r.get("tech_karma_extracted");
            let creative_rating: Option<i32> = r.try_get("creative_rating").ok();
            let execution_log: Option<String> = try_get_optional_string(&r, "execution_log");
            let error_message: Option<String> = try_get_optional_string(&r, "error_message");
            let sns_platform: Option<String> = try_get_optional_string(&r, "sns_platform");
            let sns_content_id: Option<String> = try_get_optional_string(&r, "sns_content_id");
            let published_at: Option<String> = try_get_optional_string(&r, "published_at");
            let output_artifacts: Option<String> = try_get_optional_string(&r, "output_artifacts");

            let now = Utc::now().to_rfc3339();
            sqlx::query("UPDATE jobs SET status = ?, started_at = ?, last_heartbeat = ?, updated_at = ? WHERE id = ?")
                .bind(JobStatus::Processing.to_string())
                .bind(&now)
                .bind(&now)
                .bind(&now)
                .bind(&id)
                .execute(&mut *tx)
                .await
                .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to update job status: {}", e) })?;

            tx.commit().await
                .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to commit transaction: {}", e) })?;

            Ok(Some(Job {
                id,
                category: r.get("category"),
                topic,
                style,
                karma_directives,
                status: JobStatus::Processing,
                started_at: Some(now.clone()),
                last_heartbeat: Some(now),
                tech_karma_extracted: tech_karma_extracted != 0,
                creative_rating,
                execution_log,
                error_message,
                sns_platform,
                sns_content_id,
                published_at,
                output_artifacts,
            }))
        } else {
            Ok(None)
        }
    }

    async fn do_complete_job(&self, job_id: &str, output_artifacts: Option<&str>) -> Result<(), AiomeError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE jobs SET status = ?, output_artifacts = ?, updated_at = ? WHERE id = ?")
            .bind(JobStatus::Completed.to_string())
            .bind(output_artifacts)
            .bind(&now)
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to complete job {}: {}", job_id, e) })?;
        Ok(())
    }

    async fn do_fail_job(&self, job_id: &str, reason: &str) -> Result<(), AiomeError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE jobs SET status = ?, error_message = ?, updated_at = ? WHERE id = ?")
            .bind(JobStatus::Failed.to_string())
            .bind(reason)
            .bind(&now)
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to fail job {}: {}", job_id, e) })?;
        Ok(())
    }

    async fn do_reclaim_zombie_jobs(&self, timeout_minutes: i64) -> Result<u64, AiomeError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE jobs SET status = 'Failed', error_message = 'Zombie reclaimed: heartbeat timeout exceeded', updated_at = ? 
             WHERE status = 'Processing' 
             AND last_heartbeat IS NOT NULL 
             AND (julianday('now') - julianday(last_heartbeat)) * 24 * 60 > ?"
        )
        .bind(&now)
        .bind(timeout_minutes)
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to reclaim zombie jobs: {}", e) })?;

        let count = result.rows_affected();
        if count > 0 {
            tracing::warn!("🧟 Zombie Hunter: Reclaimed {} ghost job(s)", count);
        }
        Ok(count)
    }

    async fn do_set_creative_rating(&self, job_id: &str, rating: i32) -> Result<(), AiomeError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE jobs SET creative_rating = ?, updated_at = ? WHERE id = ? AND status IN ('Completed', 'Processing')"
        )
        .bind(rating)
        .bind(&now)
        .bind(job_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to set creative rating for job {}: {}", job_id, e) })?;

        if result.rows_affected() == 0 {
            return Err(AiomeError::Infrastructure {
                reason: format!("Atomic Guard: Job '{}' is not in Completed/Processing state, rating rejected", job_id),
            });
        }
        Ok(())
    }

    async fn do_heartbeat_pulse(&self, job_id: &str) -> Result<(), AiomeError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE jobs SET last_heartbeat = ?, updated_at = ? WHERE id = ?")
            .bind(&now)
            .bind(&now)
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to pulse heartbeat for job {}: {}", job_id, e) })?;
        Ok(())
    }

    async fn do_store_execution_log(&self, job_id: &str, log: &str) -> Result<(), AiomeError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE jobs SET execution_log = ?, updated_at = ? WHERE id = ?")
            .bind(log)
            .bind(&now)
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to store execution log for job {}: {}", job_id, e) })?;
        Ok(())
    }

    async fn do_purge_old_jobs(&self, days: i64) -> Result<u64, AiomeError> {
        let result = sqlx::query(
            "DELETE FROM jobs WHERE status IN ('Completed', 'Failed') AND created_at < datetime('now', ? || ' days')"
        )
        .bind(format!("-{}", days))
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to purge old jobs: {}", e) })?;

        let purged = result.rows_affected();
        let _ = sqlx::query("PRAGMA optimize;").execute(&self.pool).await;
        Ok(purged)
    }

    async fn do_fetch_recent_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError> {
        let rows = sqlx::query(
            "SELECT id, category, topic, style_name, karma_directives, status, started_at, last_heartbeat, 
                     tech_karma_extracted, creative_rating, execution_log, error_message,
                     sns_platform, sns_content_id, published_at, output_artifacts 
              FROM jobs 
              ORDER BY created_at DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to fetch recent jobs: {}", e) })?;

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

    async fn do_get_pending_job_count(&self) -> Result<i64, AiomeError> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM jobs WHERE status = ?")
            .bind(JobStatus::Pending.to_string())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to count pending jobs: {}", e) })?;
        Ok(row.get("count"))
    }

    async fn do_get_job_count_since(&self, since: chrono::DateTime<chrono::Utc>) -> Result<i64, AiomeError> {
        let since_str = since.to_rfc3339();
        let row = sqlx::query("SELECT COUNT(*) as count FROM jobs WHERE created_at >= ?")
            .bind(since_str)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to count jobs since: {}", e) })?;
        Ok(row.get("count"))
    }

    async fn do_fetch_job_retry_count(&self, job_id: &str) -> Result<i64, AiomeError> {
        let row = sqlx::query("SELECT retry_count FROM jobs WHERE id = ?")
            .bind(job_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to fetch retry count: {}", e) })?;
        
        if let Some(r) = row {
            Ok(r.get("retry_count"))
        } else {
            Ok(0)
        }
    }

    async fn do_reset_job_retry_count(&self, job_id: &str) -> Result<(), AiomeError> {
        sqlx::query("UPDATE jobs SET retry_count = 0 WHERE id = ?")
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to reset retry count: {}", e) })?;
        Ok(())
    }

    async fn do_increment_job_retry_count(&self, job_id: &str) -> Result<bool, AiomeError> {
        let row = sqlx::query("UPDATE jobs SET retry_count = retry_count + 1 WHERE id = ? RETURNING retry_count")
            .bind(job_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to increment job retry count: {}", e) })?;
            
        let count: i64 = row.get("retry_count");
        if count >= 3 {
            sqlx::query("UPDATE jobs SET status = 'Failed', error_message = 'Poison Pill Activated: API continually fails.' WHERE id = ?")
                .bind(job_id)
                .execute(&self.pool).await.ok();
            Ok(true) 
        } else {
            Ok(false)
        }
    }

    async fn do_storage_gc(&self, threshold_gb: f64) -> Result<u64, AiomeError> {
        let threshold_bytes = (threshold_gb * 1024.0 * 1024.0 * 1024.0) as u64;
        
        // 1. Fetch all jobs with artifacts, ordered by ASC (oldest first)
        let rows = sqlx::query("SELECT id, output_artifacts FROM jobs WHERE output_artifacts IS NOT NULL AND status IN ('Completed', 'Failed') ORDER BY created_at ASC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("GC: failed to fetch artifacts: {}", e) })?;

        let mut current_total_size: u64 = 0;
        let mut job_artifacts = Vec::new();

        for row in &rows {
            let id: String = row.get("id");
            let artifacts_json: String = row.get("output_artifacts");
            if let Ok(paths) = serde_json::from_str::<Vec<String>>(&artifacts_json) {
                let mut job_size = 0;
                for p in &paths {
                    if let Ok(meta) = std::fs::metadata(p) {
                        job_size += meta.len();
                    }
                }
                current_total_size += job_size;
                job_artifacts.push((id, paths, job_size));
            }
        }

        if current_total_size <= threshold_bytes {
            return Ok(0);
        }

        tracing::info!("♻️ [StorageGC] Current storage usage ({} bytes) exceeds threshold ({} bytes). Starting cleanup.", current_total_size, threshold_bytes);

        let mut deleted_count = 0;
        let mut target_reduction = current_total_size - threshold_bytes;
        let mut reduced_so_far = 0;

        for (id, paths, size) in job_artifacts {
            if reduced_so_far >= target_reduction { break; }

            for p in paths {
                if std::fs::remove_file(&p).is_ok() {
                    deleted_count += 1;
                }
            }
            
            // Clear artifact list in DB to prevent re-scanning
            let _ = sqlx::query("UPDATE jobs SET output_artifacts = NULL WHERE id = ?").bind(&id).execute(&self.pool).await;
            
            reduced_so_far += size;
        }

        tracing::info!("♻️ [StorageGC] Cleanup complete. Removed {} artifact files.", deleted_count);
        Ok(deleted_count)
    }
}
