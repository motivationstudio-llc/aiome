/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

//! # Job Queue Tests — The Immortal Proof
//!
//! ファイルベース一時 SQLite を使った `SqliteJobQueue` の完全テストスイート。
//! 全 15 テストで心臓部の不変性を機械的に保証する。

#[cfg(test)]
mod tests {
    use crate::job_queue::SqliteJobQueue;
    use aiome_core::traits::{JobQueue, JobStatus};

    /// テスト用のユニーク一時ファイル JobQueue を作成
    /// 各テストが独自のDBファイルを持ち、ロック競合を回避する
    async fn create_test_queue() -> (SqliteJobQueue, tempfile::TempDir) {
        let tmp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let db_path = tmp_dir.path().join("test.db");
        let db_path_str = db_path.to_str().expect("Invalid path");
        let jq = SqliteJobQueue::new(db_path_str).await.expect("Failed to create test job queue");
        (jq, tmp_dir) // tmp_dir must be kept alive for the DB file to exist
    }

    // ===== 1. Basic CRUD =====

    #[tokio::test]
    async fn test_enqueue_dequeue() {
        let (jq, _tmp) = create_test_queue().await;

        let id = jq.enqueue("data_processing", "AI Future", "cinematic", Some("{}")).await.unwrap();
        assert!(!id.is_empty());

        let job = jq.dequeue(&["data_processing"]).await.unwrap();
        assert!(job.is_some());
        let job = job.unwrap();
        assert_eq!(job.id, id);
        assert_eq!(job.topic, "AI Future");
        assert_eq!(job.style, "cinematic");
        assert_eq!(job.status, JobStatus::Processing);
    }

    #[tokio::test]
    async fn test_dequeue_empty() {
        let (jq, _tmp) = create_test_queue().await;
        let job = jq.dequeue(&["data_processing"]).await.unwrap();
        assert!(job.is_none());
    }

    #[tokio::test]
    async fn test_complete_and_fail() {
        let (jq, _tmp) = create_test_queue().await;
        
        let id1 = jq.enqueue("data_processing", "Topic A", "style_a", Some("{}")).await.unwrap();
        let id2 = jq.enqueue("data_processing", "Topic B", "style_b", Some("{}")).await.unwrap();

        let _ = jq.dequeue(&["data_processing"]).await.unwrap(); // id1 -> Processing
        let _ = jq.dequeue(&["data_processing"]).await.unwrap(); // id2 -> Processing

        jq.complete_job(&id1, None).await.unwrap();
        jq.fail_job(&id2, "Test failure reason").await.unwrap();

        // Verify no more Pending jobs
        let next = jq.dequeue(&["data_processing"]).await.unwrap();
        assert!(next.is_none());
    }

    // ===== 2. Zombie Hunter =====

    #[tokio::test]
    async fn test_zombie_reclaim() {
        let (jq, _tmp) = create_test_queue().await;

        let id = jq.enqueue("data_processing", "Zombie Topic", "dark", Some("{}")).await.unwrap();
        let _ = jq.dequeue(&["data_processing"]).await.unwrap(); // Processing

        // Manually set BOTH started_at and last_heartbeat to 20 minutes ago
        sqlx::query(
            "UPDATE jobs SET started_at = datetime('now', '-20 minutes'), last_heartbeat = datetime('now', '-20 minutes') WHERE id = ?"
        )
        .bind(&id)
        .execute(jq.pool_ref())
        .await
        .unwrap();

        let reclaimed = jq.reclaim_zombie_jobs(15).await.unwrap();
        assert_eq!(reclaimed, 1);
    }

    #[tokio::test]
    async fn test_heartbeat_pulse() {
        let (jq, _tmp) = create_test_queue().await;

        let id = jq.enqueue("data_processing", "Heartbeat Test", "pulse", Some("{}")).await.unwrap();
        let _ = jq.dequeue(&["data_processing"]).await.unwrap();

        jq.heartbeat_pulse(&id).await.unwrap();
        // If heartbeat was just updated, zombie reclaim should NOT capture it
        let reclaimed = jq.reclaim_zombie_jobs(15).await.unwrap();
        assert_eq!(reclaimed, 0);
    }

    // ===== 3. Atomic Guard (Creative Rating) =====

    #[tokio::test]
    async fn test_creative_rating_success() {
        let (jq, _tmp) = create_test_queue().await;

        let id = jq.enqueue("data_processing", "Rating Test", "rated", Some("{}")).await.unwrap();
        let _ = jq.dequeue(&["data_processing"]).await.unwrap();
        jq.complete_job(&id, None).await.unwrap();

        // Completed job should accept rating
        jq.set_creative_rating(&id, 1).await.unwrap();
    }

    #[tokio::test]
    async fn test_creative_rating_guard_rejects_failed() {
        let (jq, _tmp) = create_test_queue().await;

        let id = jq.enqueue("data_processing", "Guard Test", "guarded", Some("{}")).await.unwrap();
        let _ = jq.dequeue(&["data_processing"]).await.unwrap();
        jq.fail_job(&id, "intentional failure").await.unwrap();

        // Failed job should REJECT rating (Atomic Guard)
        let result = jq.set_creative_rating(&id, 1).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Atomic Guard"), "Error should mention Atomic Guard: {}", err);
    }

    #[tokio::test]
    async fn test_creative_rating_guard_rejects_pending() {
        let (jq, _tmp) = create_test_queue().await;

        let id = jq.enqueue("data_processing", "Pending Test", "pending", Some("{}")).await.unwrap();
        // Don't dequeue — stays Pending

        let result = jq.set_creative_rating(&id, -1).await;
        assert!(result.is_err());
    }

    // ===== 4. Execution Log & Distillation =====

    #[tokio::test]
    async fn test_store_execution_log() {
        let (jq, _tmp) = create_test_queue().await;

        let id = jq.enqueue("data_processing", "Log Test", "logged", Some("{}")).await.unwrap();
        let _ = jq.dequeue(&["data_processing"]).await.unwrap();

        jq.store_execution_log(&id, "Step 1: OK
Step 2: Process
Step 3: Done").await.unwrap();
    }

    #[tokio::test]
    async fn test_fetch_undistilled() {
        let (jq, _tmp) = create_test_queue().await;

        let id = jq.enqueue("data_processing", "Undistilled", "raw", Some("{}")).await.unwrap();
        let _ = jq.dequeue(&["data_processing"]).await.unwrap();
        jq.store_execution_log(&id, "Some log output").await.unwrap();
        jq.complete_job(&id, None).await.unwrap();

        let undistilled = jq.fetch_undistilled_jobs(10).await.unwrap();
        assert_eq!(undistilled.len(), 1);
        assert_eq!(undistilled[0].id, id);
    }

    #[tokio::test]
    async fn test_mark_karma_extracted() {
        let (jq, _tmp) = create_test_queue().await;

        let id = jq.enqueue("data_processing", "Extract Test", "extract", Some("{}")).await.unwrap();
        let _ = jq.dequeue(&["data_processing"]).await.unwrap();
        jq.store_execution_log(&id, "log").await.unwrap();
        jq.complete_job(&id, None).await.unwrap();

        jq.mark_karma_extracted(&id).await.unwrap();

        let undistilled = jq.fetch_undistilled_jobs(10).await.unwrap();
        assert_eq!(undistilled.len(), 0); // Should no longer appear
    }

    // ===== 5. Karma Store & RAG =====

    #[tokio::test]
    async fn test_store_and_fetch_karma() {
        let (jq, _tmp) = create_test_queue().await;

        let id = jq.enqueue("data_processing", "Karma Test", "karma", Some("{}")).await.unwrap();
        let hash = "test_hash";
        jq.store_karma(&id, "generative_engine", "Use param X for high quality", "Technical", hash).await.unwrap();

        let results = jq.fetch_relevant_karma("Karma Test", "generative_engine", 10, hash).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].contains("Use param X"));
    }

    // ===== 6. DB Scavenger =====

    #[tokio::test]
    async fn test_purge_old_jobs() {
        let (jq, _tmp) = create_test_queue().await;

        let id = jq.enqueue("data_processing", "Old Job", "ancient", Some("{}")).await.unwrap();
        let _ = jq.dequeue(&["data_processing"]).await.unwrap();
        jq.complete_job(&id, None).await.unwrap();

        // Manually age the job by 60 days
        sqlx::query("UPDATE jobs SET created_at = datetime('now', '-60 days') WHERE id = ?")
            .bind(&id)
            .execute(jq.pool_ref())
            .await
            .unwrap();

        let purged = jq.purge_old_jobs(30).await.unwrap();
        assert_eq!(purged, 1);

        // Verify dequeue returns nothing
        let next = jq.dequeue(&["data_processing"]).await.unwrap();
        assert!(next.is_none());
    }

    #[tokio::test]
    async fn test_purge_spares_recent_jobs() {
        let (jq, _tmp) = create_test_queue().await;

        let id = jq.enqueue("data_processing", "Fresh Job", "new", Some("{}")).await.unwrap();
        let _ = jq.dequeue(&["data_processing"]).await.unwrap();
        jq.complete_job(&id, None).await.unwrap();

        // Don't age — should NOT be purged
        let purged = jq.purge_old_jobs(30).await.unwrap();
        assert_eq!(purged, 0);
    }

    // ===== 7. Invalid JSON Constraint =====

    #[tokio::test]
    async fn test_invalid_json_rejected() {
        let (jq, _tmp) = create_test_queue().await;

        // Try to enqueue with invalid JSON — should be caught by CHECK(json_valid())
        let result = jq.enqueue("data_processing", "Bad JSON", "broken", Some("NOT_VALID_JSON")).await;
        assert!(result.is_err());
    }

    // ===== 8. Concurrent Dequeue =====

    #[tokio::test]
    async fn test_concurrent_dequeue() {
        let (jq, _tmp) = create_test_queue().await;
        let jq = std::sync::Arc::new(jq);

        // Enqueue exactly 1 job
        let _id = jq.enqueue("data_processing", "Race Condition", "race", Some("{}")).await.unwrap();

        // Two concurrent dequeues — only one should get the job
        let jq1 = jq.clone();
        let jq2 = jq.clone();

        let (r1, r2) = tokio::join!(
            tokio::spawn(async move { jq1.dequeue(&["data_processing"]).await }),
            tokio::spawn(async move { jq2.dequeue(&["data_processing"]).await }),
        );

        let got1 = r1.unwrap().map(|o| o.is_some()).unwrap_or(false);
        let got2 = r2.unwrap().map(|o| o.is_some()).unwrap_or(false);

        // At least one should succeed (the other may error or get None)
        assert!(got1 || got2, "At least one dequeue should succeed: got1={}, got2={}", got1, got2);
        // They should not both succeed (exclusivity)
        assert!(!(got1 && got2), "Both dequeues should not both get the job: got1={}, got2={}", got1, got2);
    }

    // ===== 9. The Final Wire: Global Circuit Breaker =====
    #[tokio::test]
    async fn test_global_circuit_breaker() {
        let (jq, _tmp) = create_test_queue().await;
        
        // Initial state
        let fails = jq.get_global_api_failures().await.unwrap();
        assert_eq!(fails, 0);

        // Record a failure
        let new_fails = jq.record_global_api_failure().await.unwrap();
        assert_eq!(new_fails, 1);

        // Record another failure
        let new_fails = jq.record_global_api_failure().await.unwrap();
        assert_eq!(new_fails, 2);

        let fails = jq.get_global_api_failures().await.unwrap();
        assert_eq!(fails, 2);

        // Record a success
        jq.record_global_api_success().await.unwrap();

        let fails = jq.get_global_api_failures().await.unwrap();
        assert_eq!(fails, 0);
    }

    // ===== 10. Temporal Voids: Soul Versioning =====
    #[tokio::test]
    async fn test_soul_versioning_dissonance() {
        let (jq, _tmp) = create_test_queue().await;
        
        let id = jq.enqueue("data_processing", "Soul Test", "soul_style", Some("{}")).await.unwrap();
        
        let soul_v1 = "hash_v1";
        let soul_v2 = "hash_v2";

        // Store karma under Soul v1
        jq.store_karma(&id, "soul_skill", "Use this workflow", "Technical", soul_v1).await.unwrap();

        // Fetch karma using Soul v1
        let karma_v1 = jq.fetch_relevant_karma("Soul Test", "soul_skill", 10, soul_v1).await.unwrap();
        assert_eq!(karma_v1.len(), 1);
        assert!(!karma_v1[0].contains("[LEGACY KARMA"));

        // Fetch karma using Soul v2 (Simulating a Soul evolution / Cognitive Dissonance)
        let karma_v2 = jq.fetch_relevant_karma("Soul Test", "soul_skill", 10, soul_v2).await.unwrap();
        assert_eq!(karma_v2.len(), 1);
        assert!(karma_v2[0].contains("[LEGACY KARMA"));
    }
}
