/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 */

use async_trait::async_trait;
use aiome_core::error::AiomeError;
use sqlx::Row;
use std::collections::HashMap;
use super::SqliteJobQueue;

#[async_trait]
pub trait WatchtowerOps {
    async fn do_insert_chat_message(&self, channel_id: &str, role: &str, content: &str) -> Result<(), AiomeError>;
    async fn do_fetch_chat_history(&self, channel_id: &str, limit: i64) -> Result<Vec<serde_json::Value>, AiomeError>;
    async fn do_get_chat_memory_summary(&self, channel_id: &str) -> Result<Option<String>, AiomeError>;
    async fn do_update_chat_memory_summary(&self, channel_id: &str, summary: &str) -> Result<(), AiomeError>;
    async fn do_fetch_undistilled_chats_by_channel(&self) -> Result<HashMap<String, Vec<(i64, String, String)>>, AiomeError>;
    async fn do_mark_chats_as_distilled(&self, channel_id: &str, up_to_id: i64) -> Result<(), AiomeError>;
    async fn do_purge_old_distilled_chats(&self, days: i64) -> Result<u64, AiomeError>;
    async fn do_fetch_skills_for_distillation(&self, threshold: i64) -> Result<Vec<String>, AiomeError>;
    async fn do_fetch_raw_karma_for_skill(&self, skill: &str) -> Result<Vec<(String, String)>, AiomeError>;
    async fn do_apply_distilled_karma(&self, skill: &str, distilled_lesson: &str, old_karma_ids: &[String], soul_hash: &str) -> Result<(), AiomeError>;
    async fn do_adjust_karma_weight(&self, karma_id: &str, delta: i32) -> Result<(), AiomeError>;
    async fn do_karma_decay_sweep(&self) -> Result<u64, AiomeError>;
    async fn do_increment_oracle_retry_count(&self, record_id: i64) -> Result<bool, AiomeError>;
}

#[async_trait]
impl WatchtowerOps for SqliteJobQueue {
    async fn do_insert_chat_message(&self, channel_id: &str, role: &str, content: &str) -> Result<(), AiomeError> {
        sqlx::query("INSERT INTO chat_history (channel_id, role, content) VALUES (?, ?, ?)")
            .bind(channel_id)
            .bind(role)
            .bind(content)
            .execute(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to insert chat history: {}", e) })?;
        Ok(())
    }

    async fn do_fetch_chat_history(&self, channel_id: &str, limit: i64) -> Result<Vec<serde_json::Value>, AiomeError> {
        let rows = sqlx::query(
            "SELECT role, content FROM chat_history WHERE channel_id = ? ORDER BY id DESC LIMIT ?"
        )
        .bind(channel_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to fetch chat history: {}", e) })?;

        let mut messages = Vec::new();
        for row in rows {
            let role: String = row.get("role");
            let content: String = row.get("content");
            messages.push(serde_json::json!({
                "role": role,
                "content": content
            }));
        }
        messages.reverse();
        Ok(messages)
    }

    async fn do_get_chat_memory_summary(&self, channel_id: &str) -> Result<Option<String>, AiomeError> {
        let row = sqlx::query("SELECT summary FROM chat_memory_summaries WHERE channel_id = ?")
            .bind(channel_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to get chat memory summary: {}", e) })?;

        Ok(row.map(|r| r.get("summary")))
    }

    async fn do_update_chat_memory_summary(&self, channel_id: &str, summary: &str) -> Result<(), AiomeError> {
        sqlx::query(
            "INSERT INTO chat_memory_summaries (channel_id, summary, updated_at) 
             VALUES (?, ?, datetime('now'))
             ON CONFLICT(channel_id) DO UPDATE SET summary = excluded.summary, updated_at = excluded.updated_at"
        )
        .bind(channel_id)
        .bind(summary)
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to update chat memory summary: {}", e) })?;
        Ok(())
    }

    async fn do_fetch_undistilled_chats_by_channel(&self) -> Result<HashMap<String, Vec<(i64, String, String)>>, AiomeError> {
        let rows = sqlx::query(
            "SELECT id, channel_id, role, content FROM chat_history WHERE is_distilled = 0 ORDER BY channel_id ASC, id ASC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to fetch undistilled chats: {}", e) })?;

        let mut map = HashMap::new();
        for row in rows {
            let id: i64 = row.get("id");
            let channel_id: String = row.get("channel_id");
            let role: String = row.get("role");
            let content: String = row.get("content");
            map.entry(channel_id).or_insert_with(Vec::new).push((id, role, content));
        }
        Ok(map)
    }

    async fn do_mark_chats_as_distilled(&self, channel_id: &str, up_to_id: i64) -> Result<(), AiomeError> {
        sqlx::query("UPDATE chat_history SET is_distilled = 1 WHERE channel_id = ? AND id <= ?")
            .bind(channel_id)
            .bind(up_to_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to mark chats as distilled: {}", e) })?;
        Ok(())
    }

    async fn do_purge_old_distilled_chats(&self, days: i64) -> Result<u64, AiomeError> {
        let result = sqlx::query(
            "DELETE FROM chat_history WHERE is_distilled = 1 AND created_at < datetime('now', ? || ' days')"
        )
        .bind(format!("-{}", days))
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to purge old distilled chats: {}", e) })?;

        Ok(result.rows_affected())
    }

    async fn do_fetch_skills_for_distillation(&self, threshold: i64) -> Result<Vec<String>, AiomeError> {
        let rows = sqlx::query(
            "SELECT related_skill FROM karma_logs GROUP BY related_skill HAVING COUNT(id) > ?"
        )
        .bind(threshold)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to fetch skills for distillation: {}", e) })?;

        Ok(rows.into_iter().map(|r| r.get("related_skill")).collect())
    }

    async fn do_fetch_raw_karma_for_skill(&self, skill: &str) -> Result<Vec<(String, String)>, AiomeError> {
        let rows = sqlx::query(
            "SELECT id, lesson FROM karma_logs WHERE related_skill = ?"
        )
        .bind(skill)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to fetch raw karma for skill: {}", e) })?;

        Ok(rows.into_iter().map(|r| (r.get("id"), r.get("lesson"))).collect())
    }

    async fn do_adjust_karma_weight(&self, karma_id: &str, delta: i32) -> Result<(), AiomeError> {
        sqlx::query(
            "UPDATE karma_logs SET weight = MAX(0, MIN(100, weight + ?)) WHERE id = ?"
        )
        .bind(delta).bind(karma_id)
        .execute(&self.pool).await
        .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
        Ok(())
    }

    async fn do_karma_decay_sweep(&self) -> Result<u64, AiomeError> {
        let result = sqlx::query(
            "UPDATE karma_logs SET is_archived = 1
             WHERE weight < 5
               AND (last_applied_at IS NULL OR last_applied_at < datetime('now', '-90 days'))
               AND is_archived = 0"
        ).execute(&self.pool).await
        .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
        Ok(result.rows_affected())
    }

    async fn do_apply_distilled_karma(&self, skill: &str, distilled_lesson: &str, old_karma_ids: &[String], soul_hash: &str) -> Result<(), AiomeError> {
        let mut tx = self.pool.begin().await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to start tx for distillation: {}", e) })?;

        for id in old_karma_ids {
            // R2 Soft-update: Don't physically delete, mark as archived
            sqlx::query("UPDATE karma_logs SET is_archived = 1 WHERE id = ?").bind(id).execute(&mut *tx).await
                .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to archive old karma {}: {}", id, e) })?;
        }

        let new_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO karma_logs (id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at)
             VALUES (?, 'Synthesized', ?, ?, 100, ?, datetime('now'))"
        )
            .bind(&new_id)
            .bind(skill)
            .bind(distilled_lesson)
            .bind(soul_hash)
            .execute(&mut *tx)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to insert synthesized karma: {}", e) })?;

        tx.commit().await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to commit distlillation tx: {}", e) })?;

        Ok(())
    }

    async fn do_increment_oracle_retry_count(&self, record_id: i64) -> Result<bool, AiomeError> {
        let row = sqlx::query("UPDATE sns_metrics_history SET retry_count = retry_count + 1 WHERE id = ? RETURNING retry_count")
            .bind(record_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to increment oracle retry count: {}", e) })?;
            
        let count: i64 = row.get("retry_count");
        if count >= 3 {
            sqlx::query("UPDATE sns_metrics_history SET is_finalized = 1, oracle_reason = 'Poison Pill Activated: LLM Evaluation continually fails.' WHERE id = ?")
                .bind(record_id)
                .execute(&self.pool).await.ok();
            Ok(true) 
        } else {
            Ok(false)
        }
    }
}
