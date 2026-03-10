/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 */

use async_trait::async_trait;
use aiome_core::error::AiomeError;
use tracing::info;

use super::SqliteJobQueue;

#[async_trait]
pub trait DbInitializer {
    async fn init_db(&self) -> Result<(), AiomeError>;
}

#[async_trait]
impl DbInitializer for SqliteJobQueue {
    /// The Immortal Samsara Schema (完全不可侵DDL)
    async fn init_db(&self) -> Result<(), AiomeError> {
        // Use CREATE TABLE IF NOT EXISTS to prevent data loss on restart.
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY, 
                category TEXT NOT NULL,
                topic TEXT NOT NULL,
                style_name TEXT NOT NULL, 
                karma_directives TEXT NOT NULL CHECK(json_valid(karma_directives)), 
                status TEXT NOT NULL CHECK(status IN ('Pending', 'Processing', 'Completed', 'Failed')),
                started_at TEXT, 
                last_heartbeat TEXT,
                tech_karma_extracted INTEGER NOT NULL DEFAULT 0, 
                creative_rating INTEGER CHECK(creative_rating IN (-1, 0, 1)), 
                execution_log TEXT,
                error_message TEXT,
                sns_platform TEXT,
                sns_content_id TEXT,
                published_at TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to create jobs table: {}", e) })?;

        // Embedded Migrations
        for migration in [
            "ALTER TABLE jobs ADD COLUMN last_heartbeat TEXT",
            "ALTER TABLE jobs ADD COLUMN execution_log TEXT",
            "ALTER TABLE jobs ADD COLUMN sns_platform TEXT",
            "ALTER TABLE jobs ADD COLUMN sns_content_id TEXT",
            "ALTER TABLE jobs ADD COLUMN published_at TEXT",
            "ALTER TABLE jobs ADD COLUMN retry_count INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE jobs ADD COLUMN output_artifacts TEXT",
        ] {
            let _ = sqlx::query(migration).execute(&self.pool).await;
        }

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS karma_logs (
                id TEXT PRIMARY KEY,
                job_id TEXT, 
                karma_type TEXT NOT NULL CHECK(karma_type IN ('Technical', 'Creative', 'Synthesized')),
                related_skill TEXT NOT NULL, 
                lesson TEXT NOT NULL,        
                weight INTEGER NOT NULL DEFAULT 100 CHECK(weight BETWEEN 0 AND 100), 
                last_applied_at TEXT DEFAULT (datetime('now')),
                created_at TEXT DEFAULT (datetime('now')),
                FOREIGN KEY(job_id) REFERENCES jobs(id) ON DELETE SET NULL
            );"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to create karma_logs table: {}", e) })?;

        // Indices
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_jobs_status_started ON jobs(status, started_at);")
            .execute(&self.pool).await.ok();
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_karma_logs_skill_weight ON karma_logs(related_skill, weight DESC);")
            .execute(&self.pool).await.ok();
        
        // The Metrics Ledger
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS sns_metrics_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                job_id TEXT NOT NULL,
                milestone_days INTEGER NOT NULL,
                views INTEGER NOT NULL,
                likes INTEGER NOT NULL,
                comments_count INTEGER NOT NULL,
                raw_comments_json TEXT,
                oracle_score_topic REAL,
                oracle_score_visual REAL,
                oracle_score_soul REAL,
                oracle_reason TEXT,
                hard_metric_score REAL,
                engagement_rate REAL,
                is_finalized INTEGER NOT NULL DEFAULT 0,
                recorded_at TEXT DEFAULT (datetime('now')),
                FOREIGN KEY(job_id) REFERENCES jobs(id) ON DELETE CASCADE
            );"
        ).execute(&self.pool).await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to create sns_metrics_history: {}", e),
        })?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_sns_metrics_job ON sns_metrics_history(job_id, milestone_days);")
            .execute(&self.pool).await.ok();

        for migration in [
            "ALTER TABLE jobs ADD COLUMN category TEXT NOT NULL DEFAULT 'default'",
            "ALTER TABLE sns_metrics_history ADD COLUMN raw_comments_json TEXT",
            "ALTER TABLE sns_metrics_history ADD COLUMN is_finalized INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE sns_metrics_history ADD COLUMN retry_count INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE sns_metrics_history ADD COLUMN hard_metric_score REAL",
            "ALTER TABLE sns_metrics_history ADD COLUMN engagement_rate REAL",
            "ALTER TABLE sns_metrics_history ADD COLUMN alignment_score REAL",
            "ALTER TABLE sns_metrics_history ADD COLUMN growth_score REAL",
            "ALTER TABLE sns_metrics_history ADD COLUMN lesson TEXT",
            "ALTER TABLE sns_metrics_history ADD COLUMN should_evolve INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE karma_logs ADD COLUMN soul_version_hash TEXT",
            "ALTER TABLE karma_logs ADD COLUMN karma_embedding BLOB",
            "ALTER TABLE karma_logs ADD COLUMN is_federated INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE karma_logs ADD COLUMN lamport_clock INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE karma_logs ADD COLUMN node_id TEXT DEFAULT ''",
            "ALTER TABLE karma_logs ADD COLUMN signature TEXT",
            "ALTER TABLE immune_rules ADD COLUMN is_federated INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE immune_rules ADD COLUMN lamport_clock INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE immune_rules ADD COLUMN node_id TEXT DEFAULT ''",
            "ALTER TABLE immune_rules ADD COLUMN signature TEXT",
            "ALTER TABLE immune_rules ADD COLUMN status TEXT DEFAULT 'Active'",
        ] {
            let _ = sqlx::query(migration).execute(&self.pool).await;
        }
        
        // Agent Evolution Stats
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS agent_stats (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                level INTEGER NOT NULL DEFAULT 1,
                exp INTEGER NOT NULL DEFAULT 0,
                resonance INTEGER NOT NULL DEFAULT 0,
                creativity INTEGER NOT NULL DEFAULT 0,
                fatigue INTEGER NOT NULL DEFAULT 0,
                updated_at TEXT DEFAULT (datetime('now'))
            );"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to create agent_stats table: {}", e) })?;

        let _ = sqlx::query("INSERT OR IGNORE INTO agent_stats (id, level, exp, resonance, creativity, fatigue) VALUES (1, 1, 0, 0, 0, 0);")
            .execute(&self.pool)
            .await;

        // System State
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS system_state (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT DEFAULT (datetime('now'))
            );"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to create system_state table: {}", e) })?;

        let _ = sqlx::query("INSERT OR IGNORE INTO system_state (key, value) VALUES ('logical_clock', '0')")
            .execute(&self.pool).await;

        // Chat History & Memory
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS chat_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                channel_id TEXT NOT NULL,
                role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system')),
                content TEXT NOT NULL,
                is_distilled INTEGER NOT NULL DEFAULT 0,
                created_at TEXT DEFAULT (datetime('now'))
            );"
        )
        .execute(&self.pool).await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to create chat_history: {}", e) })?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_chat_history_channel ON chat_history(channel_id, created_at DESC);")
            .execute(&self.pool).await.ok();
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_chat_history_undistilled ON chat_history(is_distilled) WHERE is_distilled = 0;")
            .execute(&self.pool).await.ok();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS chat_memory_summaries (
                channel_id TEXT PRIMARY KEY,
                summary TEXT NOT NULL,
                updated_at TEXT DEFAULT (datetime('now'))
            );"
        )
        .execute(&self.pool).await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to create chat_memory_summaries: {}", e) })?;

        // Soul Mutation History
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS soul_mutation_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                old_hash TEXT NOT NULL,
                new_hash TEXT NOT NULL,
                mutation_reason TEXT NOT NULL,
                created_at TEXT DEFAULT (datetime('now'))
            );"
        )
        .execute(&self.pool).await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to create soul_mutation_history: {}", e) })?;

        // Federation Peers
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS federation_peers (
                peer_url TEXT PRIMARY KEY,
                last_sync_at TEXT NOT NULL
            );"
        )
        .execute(&self.pool).await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to create federation_peers: {}", e) })?;

        // Immune Rules & Arena History
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS immune_rules (
                id TEXT PRIMARY KEY,
                pattern TEXT NOT NULL,
                severity INTEGER NOT NULL,
                action TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'Active',
                is_federated INTEGER NOT NULL DEFAULT 0,
                lamport_clock INTEGER NOT NULL DEFAULT 0,
                node_id TEXT DEFAULT '',
                signature TEXT,
                created_at TEXT DEFAULT (datetime('now'))
            );"
        ).execute(&self.pool).await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to create immune_rules: {}", e) })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS arena_history (
                id TEXT PRIMARY KEY,
                skill_a TEXT NOT NULL,
                skill_b TEXT NOT NULL,
                topic TEXT NOT NULL,
                winner TEXT,
                reasoning TEXT,
                created_at TEXT DEFAULT (datetime('now'))
            );"
        ).execute(&self.pool).await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to create arena_history: {}", e) })?;

        // Federated Indices (Phase 15 Hardening)
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_karma_logs_federated ON karma_logs(is_federated) WHERE is_federated = 0;").execute(&self.pool).await.ok();
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_immune_rules_federated ON immune_rules(is_federated) WHERE is_federated = 0;").execute(&self.pool).await.ok();
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_karma_lamport ON karma_logs(lamport_clock, node_id);").execute(&self.pool).await.ok();
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_immune_lamport ON immune_rules(lamport_clock, node_id);").execute(&self.pool).await.ok();

        // Biome Protocol (Phase 20)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS biome_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sender_pubkey TEXT NOT NULL,
                recipient_pubkey TEXT NOT NULL,
                topic_id TEXT NOT NULL,
                content TEXT NOT NULL,
                karma_root_cid TEXT NOT NULL,
                signature TEXT NOT NULL,
                lamport_clock INTEGER NOT NULL,
                encryption TEXT NOT NULL,
                created_at TEXT DEFAULT (datetime('now'))
            );"
        ).execute(&self.pool).await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to create biome_messages: {}", e) })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS biome_peers (
                pubkey TEXT PRIMARY KEY,
                last_seen_at TEXT DEFAULT (datetime('now')),
                reputation_score INTEGER NOT NULL DEFAULT 100
            );"
        ).execute(&self.pool).await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to create biome_peers: {}", e) })?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS biome_topics (
                topic_id TEXT PRIMARY KEY,
                peer_pubkey TEXT NOT NULL,
                summary TEXT,
                status TEXT NOT NULL CHECK(status IN ('Active', 'Archived', 'Blocked')),
                turn_count INTEGER NOT NULL DEFAULT 0,
                cooldown_until TEXT,
                updated_at TEXT DEFAULT (datetime('now'))
            );"
        ).execute(&self.pool).await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to create biome_topics: {}", e) })?;

        // Evolution Chronicle (The Record of Growth)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS evolution_chronicle (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                level_at INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                description TEXT NOT NULL,
                inspiration_source TEXT,
                karma_snapshot TEXT,
                prev_record_hash TEXT NOT NULL,
                record_hash TEXT NOT NULL,
                created_at TEXT DEFAULT (datetime('now'))
            );"
        ).execute(&self.pool).await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to create evolution_chronicle: {}", e) })?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_biome_messages_recipient ON biome_messages(recipient_pubkey);").execute(&self.pool).await.ok();
        
        // CRDT Sync (Phase 20)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS timeline_checkpoints (
                id TEXT PRIMARY KEY,
                automerge_blob BLOB NOT NULL,
                last_seq INTEGER NOT NULL,
                updated_at TEXT DEFAULT (datetime('now'))
            );"
        ).execute(&self.pool).await.ok();

        info!("✅ [SqliteJobQueue] Database and migrations initialized successfully.");
        Ok(())
    }
}
