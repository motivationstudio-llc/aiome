/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 * You may not provide the software to third parties as a hosted or managed service, 
 * where the service provides users with access to any substantial set of the features 
 * or functionality of the software.
 */

use async_trait::async_trait;
use sqlx::Row;
use aiome_core::traits::{Job, JobQueue, SnsMetricsRecord};
use aiome_core::contracts::{OracleVerdict, ImmuneRule, ArenaMatch, FederatedKarma};
use aiome_core::error::AiomeError;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::time::Duration;
use chrono::Utc;
use aiome_core::llm_provider::EmbeddingProvider;
use aiome_core::traits::KarmaSearchResult;
use std::sync::Arc;
use std::collections::HashMap;
use std::time::Instant;

#[cfg(test)]
mod tests;

mod migrations;
mod core_ops;
mod karma;
mod evaluation;
mod evolution;
mod guardrails;
mod federation;
mod swarm;
mod watchtower;
mod taxonomy;
pub mod settings;
pub mod crdt;
mod expression;

use migrations::DbInitializer;
use core_ops::CoreOps;
use karma::KarmaOps;
use evaluation::EvaluationOps;
use evolution::EvolutionOps;
use guardrails::GuardrailOps;
use federation::FederationOps;
use swarm::SwarmOps;
use watchtower::WatchtowerOps;
use settings::SettingsOps;
use crdt::CrdtOps;
use expression::ExpressionOps;

/// Job Queue that utilizes SQLite in WAL Mode to allow multi-threaded queue operations.
#[derive(Clone, Debug)]
pub struct SqliteJobQueue {
    pool: SqlitePool,
    embed_provider: Arc<tokio::sync::RwLock<Option<Arc<dyn EmbeddingProvider>>>>,
    karma_cache: Arc<tokio::sync::RwLock<HashMap<String, (KarmaSearchResult, Instant)>>>,
}

impl SqliteJobQueue {
    pub fn get_pool(&self) -> &sqlx::SqlitePool {
        &self.pool
    }

    pub async fn get_embedding_provider(&self) -> Option<Arc<dyn EmbeddingProvider>> {
        self.embed_provider.read().await.clone()
    }

    /// Connects to the SQLite database and initializes the WAL mode and schema.
    pub async fn new(db_path: &str) -> Result<Self, AiomeError> {
        use std::str::FromStr;
        let options = SqliteConnectOptions::from_str(db_path)
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Invalid db_path {}: {}", db_path, e) })?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_millis(5000));

        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect_with(options)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to connect to SQLite: {}", e) })?;

        let instance = Self {
            pool,
            embed_provider: Arc::new(tokio::sync::RwLock::new(None)),
            karma_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        };

        DbInitializer::init_db(&instance).await?;
        Ok(instance)
    }

    pub async fn set_embedding_provider(&self, provider: Arc<dyn EmbeddingProvider>) {
        let mut w = self.embed_provider.write().await;
        *w = Some(provider);
    }

    pub fn with_embeddings(self, provider: Arc<dyn EmbeddingProvider>) -> Self {
        Self {
            pool: self.pool,
            embed_provider: Arc::new(tokio::sync::RwLock::new(Some(provider))),
            karma_cache: self.karma_cache,
        }
    }
}

#[async_trait]
impl JobQueue for SqliteJobQueue {
    async fn enqueue(&self, category: &str, topic: &str, style: &str, karma_directives: Option<&str>) -> Result<String, AiomeError> {
        self.do_enqueue(category, topic, style, karma_directives).await
    }

    async fn fetch_job(&self, job_id: &str) -> Result<Option<Job>, AiomeError> {
        self.do_fetch_job(job_id).await
    }

    async fn dequeue(&self, capable_categories: &[&str]) -> Result<Option<Job>, AiomeError> {
        self.do_dequeue(capable_categories).await
    }

    async fn complete_job(&self, job_id: &str, output_artifacts: Option<&str>) -> Result<(), AiomeError> {
        self.do_complete_job(job_id, output_artifacts).await
    }

    async fn fail_job(&self, job_id: &str, reason: &str) -> Result<(), AiomeError> {
        self.do_fail_job(job_id, reason).await
    }

    async fn reclaim_zombie_jobs(&self, timeout_minutes: i64) -> Result<u64, AiomeError> {
        self.do_reclaim_zombie_jobs(timeout_minutes).await
    }

    async fn set_creative_rating(&self, job_id: &str, rating: i32) -> Result<(), AiomeError> {
        self.do_set_creative_rating(job_id, rating).await
    }

    async fn heartbeat_pulse(&self, job_id: &str) -> Result<(), AiomeError> {
        self.do_heartbeat_pulse(job_id).await
    }

    async fn store_execution_log(&self, job_id: &str, log: &str) -> Result<(), AiomeError> {
        self.do_store_execution_log(job_id, log).await
    }

    async fn fetch_relevant_karma(&self, topic: &str, skill_id: &str, limit: i64, current_soul_hash: &str) -> Result<aiome_core::traits::KarmaSearchResult, AiomeError> {
        self.do_fetch_relevant_karma(topic, skill_id, limit, current_soul_hash).await
    }

    async fn store_karma(&self, job_id: &str, skill_id: &str, lesson: &str, karma_type: &str, soul_hash: &str, domain: Option<&str>, subtopic: Option<&str>) -> Result<(), AiomeError> {
        self.do_store_karma(job_id, skill_id, lesson, karma_type, soul_hash, domain, subtopic).await
    }

    async fn adjust_karma_weight(&self, karma_id: &str, delta: i32) -> Result<(), AiomeError> {
        self.do_adjust_karma_weight(karma_id, delta).await
    }

    async fn karma_decay_sweep(&self) -> Result<u64, AiomeError> {
        self.do_karma_decay_sweep().await
    }

    async fn fetch_undistilled_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError> {
        self.do_fetch_undistilled_jobs(limit).await
    }

    async fn mark_karma_extracted(&self, job_id: &str) -> Result<(), AiomeError> {
        self.do_mark_karma_extracted(job_id).await
    }

    async fn purge_old_jobs(&self, days: i64) -> Result<u64, AiomeError> {
        self.do_purge_old_jobs(days).await
    }

    async fn link_sns_data(&self, job_id: &str, platform: &str, content_id: &str) -> Result<(), AiomeError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE jobs SET sns_platform = ?, sns_content_id = ?, published_at = ?, updated_at = ? WHERE id = ?")
            .bind(platform)
            .bind(content_id)
            .bind(&now)
            .bind(&now)
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to link SNS data for job {}: {}", job_id, e) })?;
        Ok(())
    }

    async fn fetch_jobs_for_evaluation(&self, milestone_days: i64, limit: i64) -> Result<Vec<Job>, AiomeError> {
        self.do_fetch_jobs_for_evaluation(milestone_days, limit).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn record_sns_metrics(
        &self,
        job_id: &str,
        milestone_days: i64,
        views: i64,
        likes: i64,
        comments_count: i64,
        raw_comments: Option<&str>,
    ) -> Result<(), AiomeError> {
        self.do_record_sns_metrics(job_id, milestone_days, views, likes, comments_count, raw_comments).await
    }

    async fn fetch_pending_evaluations(&self, limit: i64) -> Result<Vec<SnsMetricsRecord>, AiomeError> {
        self.do_fetch_pending_evaluations(limit).await
    }

    async fn apply_final_verdict(
        &self,
        record_id: i64,
        verdict: OracleVerdict,
        soul_hash: &str,
    ) -> Result<(), AiomeError> {
        self.do_apply_final_verdict(record_id, verdict, soul_hash).await
    }

    async fn fetch_recent_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError> {
        self.do_fetch_recent_jobs(limit).await
    }

    async fn get_agent_stats(&self) -> Result<shared::watchtower::AgentStats, AiomeError> {
        self.do_get_agent_stats().await
    }

    async fn add_resonance(&self, amount: i32) -> Result<(), AiomeError> {
        self.do_add_resonance(amount).await
    }

    async fn add_tech_exp(&self, amount: i32) -> Result<(), AiomeError> {
        self.do_add_tech_exp(amount).await
    }

    async fn add_creativity(&self, amount: i32) -> Result<(), AiomeError> {
        self.do_add_creativity(amount).await
    }

    async fn sync_samsara_level(&self) -> Result<Option<aiome_core::contracts::SamsaraEvent>, AiomeError> {
        self.do_sync_samsara_level().await
    }

    async fn record_evolution_event(&self, level: i32, event_type: &str, description: &str, inspiration: Option<&str>, karma_json: Option<&str>) -> Result<(), AiomeError> {
        self.do_record_evolution_event(level, event_type, description, inspiration, karma_json).await
    }

    async fn fetch_evolution_history(&self, limit: i64) -> Result<Vec<serde_json::Value>, AiomeError> {
        self.do_fetch_evolution_history(limit).await
    }

    async fn get_pending_job_count(&self) -> Result<i64, AiomeError> {
        self.do_get_pending_job_count().await
    }

    async fn get_job_count_since(&self, since: chrono::DateTime<chrono::Utc>) -> Result<i64, AiomeError> {
        self.do_get_job_count_since(since).await
    }

    async fn fetch_all_karma(&self, limit: i64) -> Result<Vec<serde_json::Value>, AiomeError> {
        self.do_fetch_all_karma(limit).await
    }

    async fn fetch_top_performing_jobs(&self, limit: i64) -> Result<Vec<Job>, AiomeError> {
        self.do_fetch_top_performing_jobs(limit).await
    }

    async fn record_soul_mutation(&self, old_hash: &str, new_hash: &str, reason: &str) -> Result<(), AiomeError> {
        self.do_record_soul_mutation(old_hash, new_hash, reason).await
    }

    async fn fetch_job_retry_count(&self, job_id: &str) -> Result<i64, AiomeError> {
        self.do_fetch_job_retry_count(job_id).await
    }

    async fn reset_job_retry_count(&self, job_id: &str) -> Result<(), AiomeError> {
        self.do_reset_job_retry_count(job_id).await
    }

    async fn increment_job_retry_count(&self, job_id: &str) -> Result<bool, AiomeError> {
        self.do_increment_job_retry_count(job_id).await
    }

    async fn fetch_unincorporated_karma(&self, limit: i64, current_soul_hash: &str) -> Result<Vec<serde_json::Value>, AiomeError> {
        self.do_fetch_unincorporated_karma(limit, current_soul_hash).await
    }

    async fn mark_karma_as_incorporated(&self, karma_ids: Vec<String>, new_soul_hash: &str) -> Result<(), AiomeError> {
        self.do_mark_karma_as_incorporated(karma_ids, new_soul_hash).await
    }

    async fn store_immune_rule(&self, rule: &ImmuneRule) -> Result<(), AiomeError> {
        self.do_store_immune_rule(rule).await
    }

    async fn delete_immune_rule(&self, rule_id: &str) -> Result<(), AiomeError> {
        self.do_delete_immune_rule(rule_id).await
    }

    async fn fetch_active_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError> {
        self.do_fetch_active_immune_rules().await
    }

    async fn record_arena_match(&self, match_data: &ArenaMatch) -> Result<(), AiomeError> {
        self.do_record_arena_match(match_data).await
    }

    async fn export_federated_data(&self, since: Option<&str>) -> Result<(Vec<FederatedKarma>, Vec<ImmuneRule>, Vec<ArenaMatch>), AiomeError> {
        self.do_export_federated_data(since).await
    }

    async fn import_federated_data(&self, karmas: Vec<FederatedKarma>, rules: Vec<ImmuneRule>, matches: Vec<ArenaMatch>) -> Result<(), AiomeError> {
        self.do_import_federated_data(karmas, rules, matches).await
    }

    async fn get_peer_sync_time(&self, peer_url: &str) -> Result<Option<String>, AiomeError> {
        self.do_get_peer_sync_time(peer_url).await
    }

    async fn update_peer_sync_time(&self, peer_url: &str, sync_time: &str) -> Result<(), AiomeError> {
        self.do_update_peer_sync_time(peer_url, sync_time).await
    }

    async fn get_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError> {
        self.do_get_immune_rules().await
    }

    async fn get_node_id(&self) -> Result<String, AiomeError> {
        self.do_get_node_id().await
    }

    async fn sign_swarm_payload(&self, payload: &str) -> Result<String, AiomeError> {
        self.do_sign_swarm_payload(payload).await
    }

    async fn sync_local_clock(&self, remote_clock: u64) -> Result<u64, AiomeError> {
        self.do_sync_local_clock(remote_clock).await
    }

    async fn tick_local_clock(&self) -> Result<u64, AiomeError> {
        self.do_tick_local_clock().await
    }

    async fn storage_gc(&self, threshold_gb: f64) -> Result<u64, AiomeError> {
        self.do_storage_gc(threshold_gb).await
    }

    // --- Chat & Memory (The Soul Persistence) ---
    async fn store_chat_message(&self, channel_id: &str, role: &str, content: &str) -> Result<(), AiomeError> {
        self.do_insert_chat_message(channel_id, role, content).await
    }

    async fn fetch_chat_history(&self, channel_id: &str, limit: i64) -> Result<Vec<serde_json::Value>, AiomeError> {
        self.do_fetch_chat_history(channel_id, limit).await
    }

    async fn get_biome_topic_status(&self, topic_id: &str) -> Result<Option<(i32, Option<String>)>, AiomeError> {
        let row = sqlx::query("SELECT turn_count, cooldown_until FROM biome_topics WHERE topic_id = ?")
            .bind(topic_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
        
        Ok(row.map(|r| (r.get("turn_count"), r.get::<Option<String>, _>("cooldown_until"))))
    }

    async fn advance_biome_turn(&self, topic_id: &str, cooldown_minutes: i64) -> Result<i32, AiomeError> {
        let now = chrono::Utc::now();
        let cooldown_until = (now + chrono::Duration::minutes(cooldown_minutes)).to_rfc3339();
        
        let row = sqlx::query("INSERT INTO biome_topics (topic_id, turn_count, cooldown_until, updated_at) VALUES (?, 1, ?, datetime('now')) ON CONFLICT(topic_id) DO UPDATE SET turn_count = turn_count + 1, cooldown_until = excluded.cooldown_until, updated_at = excluded.updated_at RETURNING turn_count")
            .bind(topic_id)
            .bind(&cooldown_until)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
        
        Ok(row.get("turn_count"))
    }

    async fn fetch_biome_messages(&self, topic_id: &str, limit: i64) -> Result<Vec<serde_json::Value>, AiomeError> {
        let rows = sqlx::query("SELECT * FROM biome_messages WHERE topic_id = ? ORDER BY created_at DESC LIMIT ?")
            .bind(topic_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
        
        let messages = rows.into_iter().map(|row| {
            serde_json::json!({
                "id": row.get::<i64, _>("id"),
                "sender_pubkey": row.get::<String, _>("sender_pubkey"),
                "recipient_pubkey": row.get::<String, _>("recipient_pubkey"),
                "topic_id": row.get::<String, _>("topic_id"),
                "content": row.get::<String, _>("content"),
                "karma_root_cid": row.get::<String, _>("karma_root_cid"),
                "signature": row.get::<String, _>("signature"),
                "lamport_clock": row.get::<i64, _>("lamport_clock"),
                "encryption": row.get::<String, _>("encryption"),
                "created_at": row.get::<Option<String>, _>("created_at"),
            })
        }).collect();

        Ok(messages)
    }

    async fn store_biome_message(&self, message: &aiome_core::biome::BiomeMessage) -> Result<(), AiomeError> {
        sqlx::query("INSERT INTO biome_messages (sender_pubkey, recipient_pubkey, topic_id, content, karma_root_cid, signature, lamport_clock, encryption) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(&message.sender_pubkey)
            .bind(&message.recipient_pubkey)
            .bind(&message.topic_id)
            .bind(&message.content)
            .bind(&message.karma_root_cid)
            .bind(&message.signature)
            .bind(message.lamport_clock as i64)
            .bind(&message.encryption)
            .execute(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
        Ok(())
    }

    async fn update_biome_reputation(&self, pubkey: &str, delta: f64) -> Result<f64, AiomeError> {
        let row = sqlx::query("UPDATE biome_peers SET reputation_score = MAX(0, MIN(100, reputation_score + ?)) WHERE pubkey = ? RETURNING reputation_score")
            .bind(delta)
            .bind(pubkey)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
        
        Ok(row.get("reputation_score"))
    }

    // --- Expression Engine (V4) ---
    async fn store_expression(&self, expression: &aiome_core::expression::Expression) -> Result<(), AiomeError> {
        <Self as ExpressionOps>::store_expression(self, expression).await
    }

    async fn fetch_expressions(&self, limit: i64) -> Result<Vec<aiome_core::expression::Expression>, AiomeError> {
        <Self as ExpressionOps>::fetch_expressions(self, limit).await
    }

    async fn get_auto_expression_enabled(&self) -> Result<bool, AiomeError> {
        <Self as ExpressionOps>::get_auto_expression_enabled(self).await
    }

    async fn set_auto_expression_enabled(&self, enabled: bool) -> Result<(), AiomeError> {
        <Self as ExpressionOps>::set_auto_expression_enabled(self, enabled).await
    }
}

// Inherent methods (Watchtower / Chat extension)
impl SqliteJobQueue {
    pub async fn insert_chat_message(&self, channel_id: &str, role: &str, content: &str) -> Result<(), AiomeError> {
        self.do_insert_chat_message(channel_id, role, content).await
    }

    pub async fn fetch_chat_history(&self, channel_id: &str, limit: i64) -> Result<Vec<serde_json::Value>, AiomeError> {
        self.do_fetch_chat_history(channel_id, limit).await
    }

    pub async fn get_chat_memory_summary(&self, channel_id: &str) -> Result<Option<String>, AiomeError> {
        self.do_get_chat_memory_summary(channel_id).await
    }

    pub async fn update_chat_memory_summary(&self, channel_id: &str, summary: &str) -> Result<(), AiomeError> {
        self.do_update_chat_memory_summary(channel_id, summary).await
    }

    pub async fn fetch_undistilled_chats_by_channel(&self) -> Result<std::collections::HashMap<String, Vec<(i64, String, String)>>, AiomeError> {
        self.do_fetch_undistilled_chats_by_channel().await
    }

    pub async fn mark_chats_as_distilled(&self, channel_id: &str, up_to_id: i64) -> Result<(), AiomeError> {
        self.do_mark_chats_as_distilled(channel_id, up_to_id).await
    }

    pub async fn purge_old_distilled_chats(&self, days: i64) -> Result<u64, AiomeError> {
        self.do_purge_old_distilled_chats(days).await
    }

    pub async fn fetch_skills_for_distillation(&self, threshold: i64) -> Result<Vec<String>, AiomeError> {
        self.do_fetch_skills_for_distillation(threshold).await
    }

    pub async fn fetch_raw_karma_for_skill(&self, skill: &str) -> Result<Vec<(String, String)>, AiomeError> {
        self.do_fetch_raw_karma_for_skill(skill).await
    }

    pub async fn apply_distilled_karma(&self, skill: &str, distilled_lesson: &str, old_karma_ids: &[String], soul_hash: &str, domain: Option<&str>, subtopic: Option<&str>) -> Result<(), AiomeError> {
        self.do_apply_distilled_karma(skill, distilled_lesson, old_karma_ids, soul_hash, domain, subtopic).await
    }

    pub async fn increment_oracle_retry_count(&self, record_id: i64) -> Result<bool, AiomeError> {
        self.do_increment_oracle_retry_count(record_id).await
    }

    pub async fn get_global_api_failures(&self) -> Result<i64, AiomeError> {
        self.do_get_global_api_failures().await
    }

    pub async fn record_global_api_failure(&self) -> Result<i64, AiomeError> {
        self.do_record_global_api_failure().await
    }

    pub async fn record_global_api_success(&self) -> Result<(), AiomeError> {
        self.do_record_global_api_success().await
    }

    pub async fn fetch_unfederated_data(&self) -> Result<(Vec<FederatedKarma>, Vec<ImmuneRule>), AiomeError> {
        self.do_fetch_unfederated_data().await
    }

    pub async fn mark_as_federated(&self, karma_ids: Vec<String>, rule_ids: Vec<String>) -> Result<(), AiomeError> {
        self.do_mark_as_federated(karma_ids, rule_ids).await
    }

    // Settings
    pub async fn get_setting_value(&self, key: &str) -> Result<Option<String>, AiomeError> {
        self.get_setting(key).await
    }

    pub async fn update_setting(&self, key: &str, value: &str, category: &str, is_secret: bool) -> Result<(), AiomeError> {
        self.set_setting(key, value, category, is_secret).await
    }

    pub async fn fetch_all_settings(&self) -> Result<Vec<aiome_core::contracts::SystemSetting>, AiomeError> {
        self.get_all_settings().await
    }
}

// Helper function because `get` on Option panics if type is unexpected, 
// using try_get is safer if column can be NULL.
pub(crate) fn try_get_optional_string(row: &sqlx::sqlite::SqliteRow, col: &str) -> Option<String> {
    use sqlx::Row;
    row.try_get(col).ok()
}

pub(crate) fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let dot_product: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 { return 0.0; }
    dot_product / (norm_a * norm_b)
}
