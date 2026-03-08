/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 */

use async_trait::async_trait;
use aiome_core::traits::{Job, JobStatus, JobQueue};
use aiome_core::error::AiomeError;
use sqlx::Row;
use uuid::Uuid;
use chrono::Utc;
use tracing::warn;
use super::SqliteJobQueue;
use super::{try_get_optional_string, cosine_similarity};

#[async_trait]
pub trait KarmaOps {
    async fn do_fetch_relevant_karma(&self, topic: &str, skill_id: &str, limit: i64, current_soul_hash: &str) -> Result<Vec<String>, AiomeError>;
    async fn do_store_karma(&self, job_id: &str, skill_id: &str, lesson: &str, karma_type: &str, soul_hash: &str) -> Result<(), AiomeError>;
    async fn do_fetch_undistilled_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError>;
    async fn do_mark_karma_extracted(&self, job_id: &str) -> Result<(), AiomeError>;
    async fn do_fetch_all_karma(&self, limit: i64) -> Result<Vec<serde_json::Value>, AiomeError>;
    async fn do_fetch_unincorporated_karma(&self, limit: i64, current_soul_hash: &str) -> Result<Vec<serde_json::Value>, AiomeError>;
    async fn do_mark_karma_as_incorporated(&self, karma_ids: Vec<String>, new_soul_hash: &str) -> Result<(), AiomeError>;
}

#[async_trait]
impl KarmaOps for SqliteJobQueue {
    async fn do_fetch_relevant_karma(&self, topic: &str, skill_id: &str, limit: i64, current_soul_hash: &str) -> Result<Vec<String>, AiomeError> {
        let candidate_limit = limit * 5;
        let rows = sqlx::query(
            "SELECT id, lesson, soul_version_hash, karma_embedding,
              max(0, weight - (julianday('now') - julianday(created_at)) * 0.5) AS sql_weight
             FROM karma_logs 
             WHERE weight > 0 AND (related_skill = ? OR related_skill = 'global') 
             ORDER BY sql_weight DESC, created_at DESC LIMIT ?"
        )
        .bind(skill_id)
        .bind(candidate_limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("SQL Karma Query failed: {}", e) })?;

        if rows.is_empty() {
            return Ok(Vec::new());
        }

        struct KarmaCandidate {
            lesson: String,
            hash: Option<String>,
            sql_weight: f64,
            semantic_score: f64,
            stored_embedding: Option<Vec<f64>>,
        }

        let mut candidates: Vec<KarmaCandidate> = rows.iter().map(|r| {
            let embedding_bytes: Option<Vec<u8>> = r.try_get("karma_embedding").ok();
            let stored_embedding = embedding_bytes.map(|b| {
                b.chunks_exact(8)
                    .map(|chunk| f64::from_le_bytes(chunk.try_into().unwrap()))
                    .collect()
            });

            KarmaCandidate {
                lesson: r.get("lesson"),
                hash: try_get_optional_string(r, "soul_version_hash"),
                sql_weight: r.get("sql_weight"),
                semantic_score: 0.0,
                stored_embedding,
            }
        }).collect();

        if !rows.is_empty() {
            if let Some(ref provider) = self.embed_provider {
                if let Ok(topic_vec_f32) = provider.embed(topic).await {
                    let topic_vec: Vec<f64> = topic_vec_f32.into_iter().map(|f| f as f64).collect();
                    for candidate in &mut candidates {
                        if let Some(ref emb_vec) = candidate.stored_embedding {
                            candidate.semantic_score = cosine_similarity(&topic_vec, emb_vec);
                        }
                    }
                    candidates.sort_by(|a, b| {
                        let score_a = a.semantic_score * 0.7 + (a.sql_weight / 100.0) * 0.3;
                        let score_b = b.semantic_score * 0.7 + (b.sql_weight / 100.0) * 0.3;
                        score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
                    });
                } else {
                    warn!("🧬 [KarmaRAG] Failed to embed topic using {}. Falling back to SQL weight.", provider.name());
                }
            }
        }

        let mut final_karma = Vec::new();
        for candidate in candidates.into_iter().take(limit as usize) {
            let mut lesson_text = candidate.lesson;
            if let Some(h) = candidate.hash {
                if h != current_soul_hash {
                    lesson_text = format!("[LEGACY KARMA - from an older Soul version]\n{}", lesson_text);
                }
            }
            final_karma.push(lesson_text);
        }

        let now = Utc::now().to_rfc3339();
        for r in &rows {
            let id: String = r.get("id");
            let _ = sqlx::query("UPDATE karma_logs SET last_applied_at = ? WHERE id = ?").bind(&now).bind(id).execute(&self.pool).await;
        }

        Ok(final_karma)
    }

    async fn do_store_karma(&self, job_id: &str, skill_id: &str, lesson: &str, karma_type: &str, soul_hash: &str) -> Result<(), AiomeError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let node_id = self.get_node_id().await.unwrap_or_default();
        let clock = self.tick_local_clock().await.unwrap_or(0);
        let sign_target = format!("{}:{}:{}", id, lesson, clock);
        let signature = self.sign_swarm_payload(&sign_target).await.ok();

        let mut embedding: Option<Vec<u8>> = None;
        if let Some(ref provider) = self.embed_provider {
            if let Ok(vec) = provider.embed(lesson).await {
                let bytes: Vec<u8> = vec.iter().flat_map(|f| f.to_le_bytes()).collect();
                embedding = Some(bytes);
            } else {
                warn!("🧬 [KarmaStore] Failed to generate embedding using {} (ignoring)", provider.name());
            }
        }

        sqlx::query(
            "INSERT INTO karma_logs (id, job_id, karma_type, related_skill, lesson, soul_version_hash, created_at, karma_embedding, node_id, lamport_clock, signature) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(job_id)
        .bind(karma_type)
        .bind(skill_id)
        .bind(lesson)
        .bind(soul_hash)
        .bind(&now)
        .bind(embedding)
        .bind(&node_id)
        .bind(clock as i64)
        .bind(signature)
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to store karma for job {}: {}", job_id, e) })?;
        Ok(())
    }

    async fn do_fetch_undistilled_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError> {
        let rows = sqlx::query(
            "SELECT * FROM jobs 
              WHERE execution_log IS NOT NULL 
              AND tech_karma_extracted = 0 
              AND status IN ('Completed', 'Failed') 
              ORDER BY updated_at ASC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to fetch undistilled jobs: {}", e) })?;

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

    async fn do_mark_karma_extracted(&self, job_id: &str) -> Result<(), AiomeError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE jobs SET tech_karma_extracted = 1, updated_at = ? WHERE id = ?")
            .bind(&now)
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to mark karma extracted for job {}: {}", job_id, e) })?;
        Ok(())
    }

    async fn do_fetch_all_karma(&self, limit: i64) -> Result<Vec<serde_json::Value>, AiomeError> {
        let rows = sqlx::query(
            "SELECT * FROM karma_logs ORDER BY created_at DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to fetch all karma: {}", e) })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(serde_json::json!({
                "id": r.get::<String, _>("id"),
                "job_id": try_get_optional_string(&r, "job_id"),
                "skill": r.get::<String, _>("related_skill"),
                "lesson": r.get::<String, _>("lesson"),
                "karma_type": r.get::<String, _>("karma_type"),
                "weight": r.get::<i64, _>("weight"),
                "soul": try_get_optional_string(&r, "soul_version_hash"),
                "node_id": r.get::<String, _>("node_id"),
                "clock": r.get::<i64, _>("lamport_clock"),
                "signature": try_get_optional_string(&r, "signature"),
                "last_applied_at": try_get_optional_string(&r, "last_applied_at"),
                "created_at": r.get::<String, _>("created_at")
            }));
        }
        Ok(results)
    }

    async fn do_fetch_unincorporated_karma(&self, limit: i64, current_soul_hash: &str) -> Result<Vec<serde_json::Value>, AiomeError> {
        let rows = sqlx::query(
            "SELECT * FROM karma_logs 
             WHERE soul_version_hash IS NULL OR soul_version_hash != ? 
             ORDER BY created_at DESC LIMIT ?"
        )
        .bind(current_soul_hash)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to fetch unincorporated karma: {}", e) })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(serde_json::json!({
                "id": r.get::<String, _>("id"),
                "lesson": r.get::<String, _>("lesson"),
                "skill": r.get::<String, _>("related_skill"),
                "type": r.get::<String, _>("karma_type"),
                "weight": r.get::<i64, _>("weight"),
            }));
        }
        Ok(results)
    }

    async fn do_mark_karma_as_incorporated(&self, karma_ids: Vec<String>, new_soul_hash: &str) -> Result<(), AiomeError> {
        let now = Utc::now().to_rfc3339();
        for id in karma_ids {
            sqlx::query("UPDATE karma_logs SET soul_version_hash = ?, last_applied_at = ? WHERE id = ?")
                .bind(new_soul_hash)
                .bind(&now)
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to mark karma as incorporated: {}", e) })?;
        }
        Ok(())
    }
}
