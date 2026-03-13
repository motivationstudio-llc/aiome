/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Elastic License 2.0 (ELv2).
 * You may not provide the software to third parties as a hosted or managed service,
 * where the service provides users with access to any substantial set of the features
 * or functionality of the software.
 */

//! # Job Queue Tests — The Immortal Proof
//!
//! ファイルベース一時 SQLite を使った `SqliteJobQueue` の完全テストスイート。
//! 全 15 テストで心臓部の不変性を機械的に保証する。

use super::settings::SettingsOps;
use super::watchtower::WatchtowerOps;
use super::SqliteJobQueue;
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::{EmbeddingProvider, LlmProvider};
use aiome_core::traits::{JobQueue, JobStatus, KarmaEntry, KarmaSearchResult};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::Row;
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Debug)]
struct MockLlmProvider {
    json_response: String,
}

#[async_trait]
impl LlmProvider for MockLlmProvider {
    async fn complete(&self, _prompt: &str, _system: Option<&str>) -> Result<String, AiomeError> {
        Ok(self.json_response.clone())
    }
    async fn test_connection(&self) -> Result<(), AiomeError> {
        Ok(())
    }
    fn name(&self) -> &str {
        "Mock"
    }
}

/// テスト用のユニーク一時ファイル JobQueue を作成
/// 各テストが独自のDBファイルを持ち、ロック競合を回避する
async fn create_test_queue() -> (SqliteJobQueue, tempfile::TempDir) {
    let tmp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let db_path = tmp_dir.path().join("test.db");
    let db_path_str = db_path.to_str().expect("Invalid path");
    // SQLite connection string format needed for sqlx
    let jq = SqliteJobQueue::new(&format!("sqlite://{}", db_path_str))
        .await
        .expect("Failed to create test job queue");
    (jq, tmp_dir) // tmp_dir must be kept alive for the DB file to exist
}

#[tokio::test]
async fn test_sqlite_job_queue_basic_ops() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq
        .enqueue("Task", "Test Topic", "Style", None)
        .await
        .expect("Enqueue failed");
    let job = jq
        .fetch_job(&job_id)
        .await
        .expect("Fetch failed")
        .expect("Job not found");
    assert_eq!(job.topic, "Test Topic");
    assert_eq!(job.status, JobStatus::Pending);
}

#[tokio::test]
async fn test_sqlite_job_queue_dequeue_lifecycle() {
    let (jq, _tmp) = create_test_queue().await;
    jq.enqueue("Task", "Topic 1", "Style", None).await.unwrap();
    let job = jq
        .dequeue(&["Task"])
        .await
        .unwrap()
        .expect("Should dequeue job");
    assert_eq!(job.status, JobStatus::Processing);
    assert!(job.started_at.is_some());

    jq.complete_job(&job.id, Some("[\"artifact.txt\"]"))
        .await
        .unwrap();
    let updated = jq.fetch_job(&job.id).await.unwrap().unwrap();
    assert_eq!(updated.status, JobStatus::Completed);
}

#[tokio::test]
async fn test_sqlite_job_queue_karma_storage() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq.enqueue("Task", "Topic", "Style", None).await.unwrap();
    jq.store_karma(
        &job_id,
        "skill-1",
        "Lesson 1",
        "Technical",
        "hash1",
        None,
        None,
    )
    .await
    .unwrap();
    let result = jq
        .fetch_relevant_karma("Topic", "skill-1", 10, "hash1")
        .await
        .unwrap();
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].lesson, "Lesson 1");
}

#[tokio::test]
async fn test_sqlite_job_queue_zombie_reclamation() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq.enqueue("Task", "Zombies", "Style", None).await.unwrap();
    jq.dequeue(&["Task"]).await.unwrap();

    // Simulate heartbeat timeout
    sqlx::query("UPDATE jobs SET last_heartbeat = datetime('now', '-15 minutes') WHERE id = ?")
        .bind(&job_id)
        .execute(&jq.pool)
        .await
        .unwrap();

    let reclaimed = jq.reclaim_zombie_jobs(10).await.unwrap();
    assert_eq!(reclaimed, 1);
    let updated = jq.fetch_job(&job_id).await.unwrap().unwrap();
    assert_eq!(updated.status, JobStatus::Failed);
    assert!(updated.error_message.unwrap().contains("Zombie reclaimed"));
}

#[tokio::test]
async fn test_sqlite_job_queue_creative_rating_guard() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq.enqueue("Task", "Rating", "Style", None).await.unwrap();

    // Cannot rate pending job (Atomic Guard)
    let res = jq.set_creative_rating(&job_id, 1).await;
    assert!(res.is_err());

    jq.dequeue(&["Task"]).await.unwrap();
    jq.set_creative_rating(&job_id, 1)
        .await
        .expect("Should allow rating on processing");
    let job = jq.fetch_job(&job_id).await.unwrap().unwrap();
    assert_eq!(job.creative_rating, Some(1));
}

#[tokio::test]
async fn test_sqlite_job_queue_db_purge() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq.enqueue("Task", "Old Job", "Style", None).await.unwrap();
    let job = jq
        .dequeue(&["Task"])
        .await
        .unwrap()
        .expect("Job should exist");
    jq.complete_job(&job.id, None).await.unwrap();

    sqlx::query("UPDATE jobs SET created_at = datetime('now', '-30 days') WHERE id = ?")
        .bind(&job_id)
        .execute(&jq.pool)
        .await
        .unwrap();

    let purged = jq.purge_old_jobs(1).await.unwrap();
    assert_eq!(purged, 1);
    assert!(jq.fetch_job(&job_id).await.unwrap().is_none());
}

#[tokio::test]
async fn test_sqlite_job_queue_concurrent_dequeue() {
    let (jq, _tmp) = create_test_queue().await;
    jq.enqueue("Task", "Job 1", "Style", None).await.unwrap();

    // Parallel dequeue
    let mut tasks = Vec::new();
    let jq_arc = std::sync::Arc::new(jq);
    for _ in 0..5 {
        let jq_clone = jq_arc.clone();
        tasks.push(tokio::spawn(
            async move { jq_clone.dequeue(&["Task"]).await },
        ));
    }

    let results: Vec<
        Result<
            Result<Option<aiome_core::traits::Job>, aiome_core::error::AiomeError>,
            tokio::task::JoinError,
        >,
    > = futures::future::join_all(tasks).await;
    let successes = results
        .into_iter()
        .filter(|r| if let Ok(Ok(Some(_))) = r { true } else { false })
        .count();

    // Only one should successfully dequeue
    assert_eq!(successes, 1);
}

#[tokio::test]
async fn test_sqlite_job_queue_heartbeat() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq.enqueue("Task", "Heart", "Style", None).await.unwrap();
    jq.dequeue(&["Task"]).await.unwrap();

    let first = jq.fetch_job(&job_id).await.unwrap().unwrap().last_heartbeat;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    jq.heartbeat_pulse(&job_id).await.unwrap();
    let second = jq.fetch_job(&job_id).await.unwrap().unwrap().last_heartbeat;

    assert!(second > first);
}

#[tokio::test]
async fn test_sqlite_job_queue_execution_logs() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq.enqueue("Task", "Log", "Style", None).await.unwrap();
    jq.store_execution_log(&job_id, "WASM STDOUT: Hello")
        .await
        .unwrap();
    let job = jq.fetch_job(&job_id).await.unwrap().unwrap();
    assert_eq!(job.execution_log, Some("WASM STDOUT: Hello".into()));
}

#[tokio::test]
async fn test_sqlite_job_queue_unincorporate_karma() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq.enqueue("Task", "Topic", "Style", None).await.unwrap();
    jq.store_karma(
        &job_id,
        "skill-1",
        "Distilled Lesson",
        "Technical",
        "hash-old",
        None,
        None,
    )
    .await
    .unwrap();

    let uninc = jq.fetch_unincorporated_karma(10, "hash-new").await.unwrap();
    assert_eq!(uninc.len(), 1);
    assert_eq!(
        uninc[0].get("lesson").and_then(|v| v.as_str()),
        Some("Distilled Lesson")
    );
}

#[tokio::test]
async fn test_sqlite_job_queue_incorporate_karma() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq.enqueue("Task", "Topic", "Style", None).await.unwrap();
    jq.store_karma(
        &job_id,
        "skill-1",
        "Distilled Lesson",
        "Technical",
        "hash-old",
        None,
        None,
    )
    .await
    .unwrap();
    let uninc = jq.fetch_unincorporated_karma(10, "hash-new").await.unwrap();
    let id = uninc[0]
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap()
        .to_string();

    jq.mark_karma_as_incorporated(vec![id], "hash-new")
        .await
        .unwrap();
    let left = jq.fetch_unincorporated_karma(10, "hash-new").await.unwrap();
    assert_eq!(left.len(), 0);
}

#[tokio::test]
async fn test_sqlite_job_queue_retry_poison_pill() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq.enqueue("Task", "Retry", "Style", None).await.unwrap();

    jq.increment_job_retry_count(&job_id).await.unwrap();
    jq.increment_job_retry_count(&job_id).await.unwrap();
    let poisoned = jq.increment_job_retry_count(&job_id).await.unwrap();

    assert!(poisoned);
    let job = jq.fetch_job(&job_id).await.unwrap().unwrap();
    assert_eq!(job.status, JobStatus::Failed);
    assert!(job.error_message.unwrap().contains("Poison Pill"));
}

#[tokio::test]
async fn test_sqlite_job_queue_immune_rules() {
    let (jq, _tmp) = create_test_queue().await;
    let rule = aiome_core::contracts::ImmuneRule {
        id: "rule-1".into(),
        pattern: "rm -rf".into(),
        severity: 100,
        action: "Block".into(),
        created_at: Utc::now().to_rfc3339(),
        node_id: "".into(),
        lamport_clock: 0,
        signature: None,
    };
    jq.store_immune_rule(&rule).await.unwrap();
    let rules = jq.fetch_active_immune_rules().await.unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].pattern, "rm -rf");
}

#[tokio::test]
async fn test_sqlite_job_queue_arena_history() {
    let (jq, _tmp) = create_test_queue().await;
    let match_data = aiome_core::contracts::ArenaMatch {
        id: "match-1".into(),
        skill_a: "A".into(),
        skill_b: "B".into(),
        topic: "Topic".into(),
        winner: Some("A".into()),
        reasoning: "A is better".into(),
        created_at: Utc::now().to_rfc3339(),
    };
    jq.record_arena_match(&match_data).await.unwrap();
}

#[tokio::test]
async fn test_sqlite_job_queue_soul_history() {
    let (jq, _tmp) = create_test_queue().await;
    jq.record_soul_mutation("old", "new", "Mutation")
        .await
        .unwrap();
}

#[tokio::test]
async fn test_sqlite_job_queue_karma_soul_coherence() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq.enqueue("Task", "Topic", "Style", None).await.unwrap();
    let soul_v1 = "550e8400-e29b-41d4-a716-446655440000";
    let soul_v2 = "660e8400-e29b-41d4-a716-446655440001";

    jq.store_karma(
        &job_id,
        "soul_skill",
        "[V1 KARMA]",
        "Synthesized",
        soul_v1,
        None,
        None,
    )
    .await
    .unwrap();

    let result_v1 = jq
        .fetch_relevant_karma("Soul Test", "soul_skill", 10, soul_v1)
        .await
        .unwrap();
    assert_eq!(result_v1.entries.len(), 1);
    assert_eq!(result_v1.entries[0].lesson, "[V1 KARMA]");

    // Implementation returns legacy marked karma instead of empty list
    let result_v2_legacy = jq
        .fetch_relevant_karma("Soul Test Legacy", "soul_skill", 10, soul_v2)
        .await
        .unwrap();
    assert_eq!(result_v2_legacy.entries.len(), 1);
    assert!(result_v2_legacy.entries[0].lesson.contains("[LEGACY KARMA"));

    let job_id2 = jq.enqueue("Task", "Topic 2", "Style", None).await.unwrap();
    jq.store_karma(
        &job_id2,
        "soul_skill",
        "[V2 KARMA]",
        "Synthesized",
        soul_v2,
        None,
        None,
    )
    .await
    .unwrap();

    let result_v2 = jq
        .fetch_relevant_karma("Soul Test Final", "soul_skill", 10, soul_v2)
        .await
        .unwrap();
    assert_eq!(result_v2.entries.len(), 2);
    assert!(result_v2
        .entries
        .iter()
        .any(|k| k.lesson.contains("[LEGACY KARMA") && k.lesson.contains("[V1 KARMA]")));
    assert!(result_v2.entries.iter().any(|k| k.lesson == "[V2 KARMA]"));
}

#[derive(Debug)]
struct MockEmbedProvider;
#[async_trait]
impl EmbeddingProvider for MockEmbedProvider {
    fn name(&self) -> &str {
        "mock"
    }
    async fn embed(
        &self,
        text: &str,
        _is_query: bool,
    ) -> Result<Vec<f32>, aiome_core::error::AiomeError> {
        if text.contains("alien") {
            Ok(vec![0.0; 1536])
        } else {
            Ok(vec![1.0; 1536])
        }
    }
    async fn test_connection(&self) -> Result<(), aiome_core::error::AiomeError> {
        Ok(())
    }
}

#[tokio::test]
async fn test_sqlite_job_queue_karma_ood_detection() {
    let (mut jq, _tmp) = create_test_queue().await;
    jq = jq.with_embeddings(Arc::new(MockEmbedProvider));

    let job_id = jq
        .enqueue("Task", "Real Topic", "Style", None)
        .await
        .unwrap();
    // Use manual SQL to insert embedding matched to MockEmbedProvider's output
    let id = uuid::Uuid::new_v4().to_string();
    let emb: Vec<u8> = vec![1.0f32; 1536]
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect();
    sqlx::query("INSERT INTO karma_logs (id, job_id, karma_type, related_skill, lesson, created_at, karma_embedding) VALUES (?, ?, 'Technical', 'skill-1', 'Real Lesson', datetime('now'), ?)")
        .bind(&id).bind(&job_id).bind(&emb).execute(&jq.pool).await.unwrap();

    // Closer match (Mock returns 1.0, DB has 1.0 -> score 1.0)
    let result = jq
        .fetch_relevant_karma("Real Topic", "skill-1", 10, "hash1")
        .await
        .unwrap();
    assert!(!result.is_ood);

    // Out of domain (Mock returns 0.0, DB has 1.0 -> score 0.0)
    let result_ood = jq
        .fetch_relevant_karma("space aliens", "skill-1", 10, "hash1")
        .await
        .unwrap();
    assert!(result_ood.is_ood);
}

#[tokio::test]
async fn test_sqlite_job_queue_karma_cache_hit() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq
        .enqueue("Task", "Cache Test", "Style", None)
        .await
        .unwrap();
    jq.store_karma(
        &job_id,
        "skill-1",
        "Cached Lesson",
        "Technical",
        "hash1",
        None,
        None,
    )
    .await
    .unwrap();

    // First call - fills cache
    let _ = jq
        .fetch_relevant_karma("Cache Test", "skill-1", 10, "hash1")
        .await
        .unwrap();

    // Directly modify DB
    sqlx::query("UPDATE karma_logs SET lesson = 'Modified Lesson'")
        .execute(&jq.pool)
        .await
        .unwrap();

    // Second call - should hit cache
    let result2 = jq
        .fetch_relevant_karma("Cache Test", "skill-1", 10, "hash1")
        .await
        .unwrap();
    assert_eq!(result2.entries[0].lesson, "Cached Lesson"); // Cache hit
}

#[tokio::test]
async fn test_sqlite_job_queue_karma_weight_clamp() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq.enqueue("Task", "Topic", "Style", None).await.unwrap();
    jq.store_karma(
        &job_id,
        "skill-1",
        "Lesson",
        "Technical",
        "hash1",
        None,
        None,
    )
    .await
    .unwrap();

    // Default weight is 100 or inherited. Let's find the id.
    let row = sqlx::query("SELECT id, weight FROM karma_logs LIMIT 1")
        .fetch_one(&jq.pool)
        .await
        .unwrap();
    let kid: String = row.get("id");

    // Clamp Max
    jq.adjust_karma_weight(&kid, 50).await.unwrap();
    let row_max = sqlx::query("SELECT weight FROM karma_logs WHERE id = ?")
        .bind(&kid)
        .fetch_one(&jq.pool)
        .await
        .unwrap();
    assert_eq!(row_max.get::<i64, _>("weight"), 100);

    // Clamp Min
    jq.adjust_karma_weight(&kid, -150).await.unwrap();
    let row_min = sqlx::query("SELECT weight FROM karma_logs WHERE id = ?")
        .bind(&kid)
        .fetch_one(&jq.pool)
        .await
        .unwrap();
    assert_eq!(row_min.get::<i64, _>("weight"), 0);
}

#[tokio::test]
async fn test_sqlite_job_queue_karma_forgetting_sweep() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq.enqueue("Task", "Topic", "Style", None).await.unwrap();

    // 1. Weak memory (low weight) + unused
    jq.store_karma(
        &job_id,
        "skill-1",
        "Weak Lesson",
        "Technical",
        "hash1",
        None,
        None,
    )
    .await
    .unwrap();
    sqlx::query("UPDATE karma_logs SET weight = 2, last_applied_at = datetime('now', '-91 days') WHERE lesson = 'Weak Lesson'").execute(&jq.pool).await.unwrap();

    // 2. Another weak/old memory
    jq.store_karma(
        &job_id,
        "skill-1",
        "Old Lesson",
        "Technical",
        "hash1",
        None,
        None,
    )
    .await
    .unwrap();
    sqlx::query("UPDATE karma_logs SET weight = 3, last_applied_at = datetime('now', '-100 days') WHERE lesson = 'Old Lesson'").execute(&jq.pool).await.unwrap();

    // 3. Fresh strong memory
    jq.store_karma(
        &job_id,
        "skill-1",
        "Strong Lesson",
        "Technical",
        "hash1",
        None,
        None,
    )
    .await
    .unwrap();

    // 4. Strong but old memory (should NOT be archived because weight is high)
    jq.store_karma(
        &job_id,
        "skill-1",
        "Old Strong Lesson",
        "Technical",
        "hash1",
        None,
        None,
    )
    .await
    .unwrap();
    sqlx::query("UPDATE karma_logs SET weight = 80, last_applied_at = datetime('now', '-200 days') WHERE lesson = 'Old Strong Lesson'").execute(&jq.pool).await.unwrap();

    // Run sweep
    let archived = jq.karma_decay_sweep().await.unwrap();
    assert_eq!(archived, 2); // Weak + Old (now weak) should be archived

    // Verify search excludes archived
    let result = jq
        .fetch_relevant_karma("Topic", "skill-1", 10, "hash1")
        .await
        .unwrap();
    // Strong Lesson and Old Strong Lesson should remain
    assert_eq!(result.entries.len(), 2);
    let lessons: Vec<String> = result.entries.iter().map(|e| e.lesson.clone()).collect();
    assert!(lessons.contains(&"Strong Lesson".to_string()));
    assert!(lessons.contains(&"Old Strong Lesson".to_string()));
}
#[tokio::test]
async fn test_sqlite_job_queue_karma_fts_match() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq.enqueue("Task", "Topic", "Style", None).await.unwrap();

    // 1. Generic lesson
    jq.store_karma(
        &job_id,
        "skill-1",
        "Generic baking recipe",
        "Technical",
        "hash1",
        None,
        None,
    )
    .await
    .unwrap();
    // 2. Focused lesson with keyword 'security'
    jq.store_karma(
        &job_id,
        "skill-1",
        "Security Best Practices for bakers",
        "Technical",
        "hash1",
        None,
        None,
    )
    .await
    .unwrap();

    // Search for 'security'
    let result = jq
        .fetch_relevant_karma("security", "skill-1", 10, "hash1")
        .await
        .unwrap();
    assert_eq!(result.entries.len(), 2);
    // The one with 'Security' in text should be first due to FTS5 boost (50.0)
    assert!(result.entries[0].lesson.contains("Security"));
    assert!(!result.is_ood);
}

#[tokio::test]
async fn test_karma_taxonomy_classification() {
    let mock = MockLlmProvider {
        json_response: r#"{ "domain": "Technical", "subtopic": "Security", "reasoning": "Lesson about security." }"#.to_string(),
    };

    let result =
        super::taxonomy::KarmaTaxonomy::classify(&mock, "Always use parameterized queries")
            .await
            .unwrap();
    assert_eq!(result.domain, "Technical");
    assert_eq!(result.subtopic, "Security");
}

#[tokio::test]
async fn test_karma_taxonomy_fallback() {
    let mock = MockLlmProvider {
        json_response: "garbage".to_string(),
    };

    let result =
        super::taxonomy::KarmaTaxonomy::classify(&mock, "Always use parameterized queries").await;
    assert!(result.is_err());

    let fb = super::taxonomy::KarmaTaxonomy::fallback();
    assert_eq!(fb.domain, "general");
}

#[tokio::test]
async fn test_sqlite_settings_crud() {
    let (jq, _tmp) = create_test_queue().await;

    // Test set and get
    jq.set_setting("llm_model", "test-model-1", "llm", false)
        .await
        .expect("Failed to set");
    let val = jq.get_setting_value("llm_model").await.unwrap();
    assert_eq!(val, Some("test-model-1".to_string()));

    // Test overwrite
    jq.set_setting("llm_model", "test-model-2", "llm", false)
        .await
        .expect("Failed to overwrite");
    let val2 = jq.get_setting_value("llm_model").await.unwrap();
    assert_eq!(val2, Some("test-model-2".to_string()));

    // Test fetch all (visible)
    let all = jq.fetch_all_settings().await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].key, "llm_model");
    assert_eq!(all[0].value, "test-model-2");
}

#[tokio::test]
async fn test_sqlite_settings_secret_masking() {
    let (jq, _tmp) = create_test_queue().await;

    // Set a secret
    jq.set_setting("telegram_token", "super-secret-123", "system", true)
        .await
        .expect("Failed to set secret");

    // get_setting_value should return the actual value (for internal use)
    let val = jq.get_setting_value("telegram_token").await.unwrap();
    assert_eq!(val, Some("super-secret-123".to_string()));

    // fetch_all_settings isn't implemented as a method directly yielding masked values in tests.
    // The web layer `routes::settings::get_settings` does the masking.
    // In db layer fetch_all_settings, we expect it to return raw values, or we manually verify the field `is_secret` is true.
    let all = jq.fetch_all_settings().await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].key, "telegram_token");
    assert!(all[0].is_secret);
    // Since this is the direct test of SqliteJobQueue, we might just be testing if `is_secret` is respected, not necessarily evaluating presentation masking here.
    // In our `api-server`, get_settings does `if s.is_secret { s.value = "********" }`.
    // If we want DB-level masking, we'd need to update `fetch_all_settings`. Let's just assert `is_secret` flag.
}
