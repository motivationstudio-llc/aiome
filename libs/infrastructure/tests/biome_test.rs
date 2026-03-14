/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

use aiome_core::biome::BiomeMessage;
use aiome_core::traits::JobQueue;
use infrastructure::job_queue::SqliteJobQueue;

#[tokio::test]
async fn test_biome_dialogue_limit() {
    let queue = SqliteJobQueue::new("sqlite::memory:").await.unwrap();
    let topic_id = "test_dialogue_topic";

    // Simulate 10 turns
    for i in 0..10 {
        let count = queue.advance_biome_turn(topic_id, 0).await.unwrap();
        assert_eq!(count, i + 1);

        let msg = BiomeMessage {
            sender_pubkey: "peer_a".to_string(),
            recipient_pubkey: "peer_b".to_string(),
            topic_id: topic_id.to_string(),
            content: format!("Msg {}", i),
            karma_root_cid: "cid".to_string(),
            signature: "sig".to_string(),
            lamport_clock: i as u64,
            timestamp: chrono::Utc::now().to_rfc3339(),
            encryption: "none".to_string(),
        };
        queue.store_biome_message(&msg).await.unwrap();
    }

    let status = queue
        .get_biome_topic_status(topic_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(status.0, 10); // 10 turns reached

    // Archive it
    queue.archive_biome_topic(topic_id).await.unwrap();

    // Verify it's archived
    let archived_status: String =
        sqlx::query_scalar("SELECT status FROM biome_topics WHERE topic_id = ?")
            .bind(topic_id)
            .fetch_one(queue.get_pool())
            .await
            .unwrap();

    assert_eq!(archived_status, "Archived");
}
