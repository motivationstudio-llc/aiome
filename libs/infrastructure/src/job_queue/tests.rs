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

use super::SqliteJobQueue;
use aiome_core::traits::{JobQueue, JobStatus};
use chrono::Utc;

/// テスト用のユニーク一時ファイル JobQueue を作成
/// 各テストが独自のDBファイルを持ち、ロック競合を回避する
async fn create_test_queue() -> (SqliteJobQueue, tempfile::TempDir) {
    let tmp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let db_path = tmp_dir.path().join("test.db");
    let db_path_str = db_path.to_str().expect("Invalid path");
    // SQLite connection string format needed for sqlx
    let jq = SqliteJobQueue::new(&format!("sqlite://{}", db_path_str)).await.expect("Failed to create test job queue");
    (jq, tmp_dir) // tmp_dir must be kept alive for the DB file to exist
}

#[tokio::test]
async fn test_sqlite_job_queue_basic_ops() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq.enqueue("Task", "Test Topic", "Style", None).await.expect("Enqueue failed");
    let job = jq.fetch_job(&job_id).await.expect("Fetch failed").expect("Job not found");
    assert_eq!(job.topic, "Test Topic");
    assert_eq!(job.status, JobStatus::Pending);
}

#[tokio::test]
async fn test_sqlite_job_queue_dequeue_lifecycle() {
    let (jq, _tmp) = create_test_queue().await;
    jq.enqueue("Task", "Topic 1", "Style", None).await.unwrap();
    let job = jq.dequeue(&["Task"]).await.unwrap().expect("Should dequeue job");
    assert_eq!(job.status, JobStatus::Processing);
    assert!(job.started_at.is_some());

    jq.complete_job(&job.id, Some("[\"artifact.txt\"]")).await.unwrap();
    let updated = jq.fetch_job(&job.id).await.unwrap().unwrap();
    assert_eq!(updated.status, JobStatus::Completed);
}

#[tokio::test]
async fn test_sqlite_job_queue_karma_storage() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq.enqueue("Task", "Topic", "Style", None).await.unwrap();
    jq.store_karma(&job_id, "skill-1", "Lesson 1", "Technical", "hash1").await.unwrap();
    let karma = jq.fetch_relevant_karma("Topic", "skill-1", 10, "hash1").await.unwrap();
    assert_eq!(karma.len(), 1);
    assert_eq!(karma[0], "Lesson 1");
}

#[tokio::test]
async fn test_sqlite_job_queue_zombie_reclamation() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq.enqueue("Task", "Zombies", "Style", None).await.unwrap();
    jq.dequeue(&["Task"]).await.unwrap();
    
    // Simulate heartbeat timeout
    sqlx::query("UPDATE jobs SET last_heartbeat = datetime('now', '-15 minutes') WHERE id = ?")
        .bind(&job_id).execute(&jq.pool).await.unwrap();
    
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
    jq.set_creative_rating(&job_id, 1).await.expect("Should allow rating on processing");
    let job = jq.fetch_job(&job_id).await.unwrap().unwrap();
    assert_eq!(job.creative_rating, Some(1));
}

#[tokio::test]
async fn test_sqlite_job_queue_db_purge() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq.enqueue("Task", "Old Job", "Style", None).await.unwrap();
    let job = jq.dequeue(&["Task"]).await.unwrap().expect("Job should exist");
    jq.complete_job(&job.id, None).await.unwrap();
    
    sqlx::query("UPDATE jobs SET created_at = datetime('now', '-30 days') WHERE id = ?")
        .bind(&job_id).execute(&jq.pool).await.unwrap();
    
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
        tasks.push(tokio::spawn(async move {
            jq_clone.dequeue(&["Task"]).await
        }));
    }
    
    let results: Vec<Result<Result<Option<aiome_core::traits::Job>, aiome_core::error::AiomeError>, tokio::task::JoinError>> = futures::future::join_all(tasks).await;
    let successes = results.into_iter().filter(|r| {
        if let Ok(Ok(Some(_))) = r {
            true
        } else {
            false
        }
    }).count();
    
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
    jq.store_execution_log(&job_id, "WASM STDOUT: Hello").await.unwrap();
    let job = jq.fetch_job(&job_id).await.unwrap().unwrap();
    assert_eq!(job.execution_log, Some("WASM STDOUT: Hello".into()));
}

#[tokio::test]
async fn test_sqlite_job_queue_unincorporate_karma() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq.enqueue("Task", "Topic", "Style", None).await.unwrap();
    jq.store_karma(&job_id, "skill-1", "Distilled Lesson", "Technical", "hash-old").await.unwrap();
    
    let uninc = jq.fetch_unincorporated_karma(10, "hash-new").await.unwrap();
    assert_eq!(uninc.len(), 1);
    assert_eq!(uninc[0].get("lesson").and_then(|v| v.as_str()), Some("Distilled Lesson"));
}

#[tokio::test]
async fn test_sqlite_job_queue_incorporate_karma() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq.enqueue("Task", "Topic", "Style", None).await.unwrap();
    jq.store_karma(&job_id, "skill-1", "Distilled Lesson", "Technical", "hash-old").await.unwrap();
    let uninc = jq.fetch_unincorporated_karma(10, "hash-new").await.unwrap();
    let id = uninc[0].get("id").and_then(|v| v.as_str()).unwrap().to_string();
    
    jq.mark_karma_as_incorporated(vec![id], "hash-new").await.unwrap();
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
    jq.record_soul_mutation("old", "new", "Mutation").await.unwrap();
}

#[tokio::test]
async fn test_sqlite_job_queue_karma_soul_coherence() {
    let (jq, _tmp) = create_test_queue().await;
    let job_id = jq.enqueue("Task", "Topic", "Style", None).await.unwrap();
    let soul_v1 = "550e8400-e29b-41d4-a716-446655440000";
    let soul_v2 = "660e8400-e29b-41d4-a716-446655440001";
    
    jq.store_karma(&job_id, "soul_skill", "[V1 KARMA]", "Synthesized", soul_v1).await.unwrap();
    
    let karma_v1 = jq.fetch_relevant_karma("Soul Test", "soul_skill", 10, soul_v1).await.unwrap();
    assert_eq!(karma_v1.len(), 1);
    assert_eq!(karma_v1[0], "[V1 KARMA]");

    // Implementation returns legacy marked karma instead of empty list
    let karma_v2_legacy = jq.fetch_relevant_karma("Soul Test", "soul_skill", 10, soul_v2).await.unwrap();
    assert_eq!(karma_v2_legacy.len(), 1);
    assert!(karma_v2_legacy[0].contains("[LEGACY KARMA"));

    let job_id2 = jq.enqueue("Task", "Topic 2", "Style", None).await.unwrap();
    jq.store_karma(&job_id2, "soul_skill", "[V2 KARMA]", "Synthesized", soul_v2).await.unwrap();
    
    let karma_v2 = jq.fetch_relevant_karma("Soul Test", "soul_skill", 10, soul_v2).await.unwrap();
    assert_eq!(karma_v2.len(), 2); 
    assert!(karma_v2.iter().any(|k| k.contains("[LEGACY KARMA") && k.contains("[V1 KARMA]")));
    assert!(karma_v2.iter().any(|k| k == "[V2 KARMA]"));
}
