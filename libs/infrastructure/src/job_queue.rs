use rig::providers::gemini;
use rig::prelude::*;
use async_trait::async_trait;
use factory_core::traits::{Job, JobQueue, JobStatus, SnsMetricsRecord};
use factory_core::contracts::OracleVerdict;
use factory_core::error::FactoryError;
use sqlx::{SqlitePool, Row};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::time::Duration;
use uuid::Uuid;
use chrono::Utc;
use tracing::warn;

/// Job Queue that utilizes SQLite in WAL Mode to allow multi-threaded queue operations.
/// Implements **The Immortal Samsara Schema** — crash-resistant, self-healing, and eternal.
#[derive(Clone)]
pub struct SqliteJobQueue {
    pool: SqlitePool,
    gemini_api_key: Option<String>,
}

impl SqliteJobQueue {
    /// Connects to the SQLite database and initializes the WAL mode and schema.
    pub async fn new(db_path: &str) -> Result<Self, FactoryError> {
        use std::str::FromStr;
        let options = SqliteConnectOptions::from_str(db_path)
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Invalid db_path {}: {}", db_path, e) })?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_millis(5000));

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to connect to SQLite: {}", e) })?;

        let queue = Self { pool, gemini_api_key: None };
        queue.init_db().await?;
        Ok(queue)
    }

    /// Add embedding capability to the queue
    pub fn with_embeddings(mut self, api_key: &str) -> Self {
        self.gemini_api_key = Some(api_key.to_string());
        self
    }

    /// Read-only reference to the connection pool (for advanced queries).
    pub fn pool_ref(&self) -> &SqlitePool {
        &self.pool
    }

    /// The Immortal Samsara Schema (完全不可侵DDL)
    /// 
    /// Guardrails implemented at the DB level:
    /// - `CHECK(json_valid(karma_directives))`: Native JSON validation (罠3 防衛)
    /// - `started_at`: Zombie Process detection (The Zombie Hunter)
    /// - `ON DELETE SET NULL`: Eternal Karma — jobs die, lessons live (The Memory Wipe Trap 防衛)
    /// - `CHECK(weight BETWEEN 0 AND 100)`: Bounded Confidence (The Karma Singularity 防衛)
    /// - `last_applied_at`: Usage tracking for TTL decay (The Static Decay Trap 防衛)
    async fn init_db(&self) -> Result<(), FactoryError> {
        // Use CREATE TABLE IF NOT EXISTS to prevent data loss on restart.
        // The old DROP TABLE approach is replaced for production safety.
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY, 
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
                sns_video_id TEXT,
                published_at TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to create jobs table: {}", e) })?;

        // Embedded Migrations: safely add columns that may not exist in older schemas.
        // SQLite ALTER TABLE ADD COLUMN errors are silently ignored (idempotent).
        for migration in [
            "ALTER TABLE jobs ADD COLUMN last_heartbeat TEXT",
            "ALTER TABLE jobs ADD COLUMN execution_log TEXT",
            "ALTER TABLE jobs ADD COLUMN sns_platform TEXT",
            "ALTER TABLE jobs ADD COLUMN sns_video_id TEXT",
            "ALTER TABLE jobs ADD COLUMN published_at TEXT",
            "ALTER TABLE jobs ADD COLUMN retry_count INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE jobs ADD COLUMN output_videos TEXT",
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
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to create karma_logs table: {}", e) })?;

        // Indices for optimal performance
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_jobs_status_started ON jobs(status, started_at);")
            .execute(&self.pool).await.ok();
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_karma_logs_skill_weight ON karma_logs(related_skill, weight DESC);")
            .execute(&self.pool).await.ok();
        
        // The Metrics Ledger (評価台帳)
        // Stores chronological snapshots of SNS performance at milestones (24h, 7d, 30d).
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
        ).execute(&self.pool).await.map_err(|e| FactoryError::Infrastructure {
            reason: format!("Failed to create sns_metrics_history: {}", e),
        })?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_sns_metrics_job ON sns_metrics_history(job_id, milestone_days);")
            .execute(&self.pool).await.ok();

        // New migrations for sns_metrics_history refinement
        for migration in [
            "ALTER TABLE sns_metrics_history ADD COLUMN raw_comments_json TEXT",
            "ALTER TABLE sns_metrics_history ADD COLUMN is_finalized INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE sns_metrics_history ADD COLUMN retry_count INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE sns_metrics_history ADD COLUMN hard_metric_score REAL",
            "ALTER TABLE sns_metrics_history ADD COLUMN engagement_rate REAL",
            "ALTER TABLE karma_logs ADD COLUMN soul_version_hash TEXT",
            "ALTER TABLE karma_logs ADD COLUMN karma_embedding BLOB",
            "ALTER TABLE karma_logs ADD COLUMN is_federated INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE immune_rules ADD COLUMN is_federated INTEGER NOT NULL DEFAULT 0",
        ] {
            let _ = sqlx::query(migration).execute(&self.pool).await;
        }
        
        // --- Phase 12: Project Ani Foundation ---
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS agent_stats (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                level INTEGER NOT NULL DEFAULT 1,
                exp INTEGER NOT NULL DEFAULT 0,
                affection INTEGER NOT NULL DEFAULT 0,
                intimacy INTEGER NOT NULL DEFAULT 0,
                fatigue INTEGER NOT NULL DEFAULT 0,
                updated_at TEXT DEFAULT (datetime('now'))
            );"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to create agent_stats table: {}", e) })?;

        // Seed initial data if table is empty
        let _ = sqlx::query("INSERT OR IGNORE INTO agent_stats (id, level, exp, affection, intimacy, fatigue) VALUES (1, 1, 0, 0, 0, 0);")
            .execute(&self.pool)
            .await;

        // The Temporal Voids protection: Global Circuit Breaker State
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS system_state (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT DEFAULT (datetime('now'))
            );"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to create system_state table: {}", e) })?;

        // --- Watchtower Memory Distillation Tables ---
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
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to create chat_history: {}", e) })?;

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
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to create chat_memory_summaries: {}", e) })?;

        // --- Phase 5: Transmigration History ---
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
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to create soul_mutation_history: {}", e) })?;

        // --- Phase 12-F: Karma Federation ---
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS federation_peers (
                peer_url TEXT PRIMARY KEY,
                last_sync_at TEXT NOT NULL
            );"
        )
        .execute(&self.pool).await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to create federation_peers: {}", e) })?;

        Ok(())
    }
}

#[async_trait]
impl JobQueue for SqliteJobQueue {
    async fn enqueue(&self, topic: &str, style: &str, karma_directives: Option<&str>) -> Result<String, FactoryError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        // Default to empty JSON object if None, satisfying CHECK(json_valid(...))
        let directives = karma_directives.unwrap_or("{}");

        sqlx::query(
            "INSERT INTO jobs (id, topic, style_name, karma_directives, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(topic)
        .bind(style)
        .bind(directives)
        .bind(JobStatus::Pending.to_string())
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to enqueue job: {}", e) })?;

        Ok(id)
    }

    async fn fetch_job(&self, job_id: &str) -> Result<Option<Job>, FactoryError> {
        let row = sqlx::query(
            "SELECT id, topic, style_name, karma_directives, status, started_at, last_heartbeat, tech_karma_extracted, creative_rating, execution_log, error_message, sns_platform, sns_video_id, published_at, output_videos FROM jobs WHERE id = ?"
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to fetch job {}: {}", job_id, e) })?;

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
            let sns_video_id: Option<String> = try_get_optional_string(&r, "sns_video_id");
            let published_at: Option<String> = try_get_optional_string(&r, "published_at");
            let output_videos: Option<String> = try_get_optional_string(&r, "output_videos");
            let status_str: String = r.get("status");
            let status = JobStatus::from_string(&status_str);

            Ok(Some(Job {
                id,
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
                sns_video_id,
                published_at,
                output_videos,
            }))
        } else {
            Ok(None)
        }
    }

    async fn dequeue(&self) -> Result<Option<Job>, FactoryError> {
        let mut tx = self.pool.begin().await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to start transaction: {}", e) })?;

        let row = sqlx::query(
            "SELECT id, topic, style_name, karma_directives, status, started_at, last_heartbeat, tech_karma_extracted, creative_rating, execution_log, error_message, sns_platform, sns_video_id, published_at, output_videos FROM jobs WHERE status = ? ORDER BY created_at ASC LIMIT 1"
        )
        .bind(JobStatus::Pending.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to fetch pending job: {}", e) })?;

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
            let sns_video_id: Option<String> = try_get_optional_string(&r, "sns_video_id");
            let published_at: Option<String> = try_get_optional_string(&r, "published_at");
            let output_videos: Option<String> = try_get_optional_string(&r, "output_videos");

            let now = Utc::now().to_rfc3339();
            // Set status to Processing, record started_at AND first heartbeat
            sqlx::query("UPDATE jobs SET status = ?, started_at = ?, last_heartbeat = ?, updated_at = ? WHERE id = ?")
                .bind(JobStatus::Processing.to_string())
                .bind(&now)
                .bind(&now)
                .bind(&now)
                .bind(&id)
                .execute(&mut *tx)
                .await
                .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to update job status: {}", e) })?;

            tx.commit().await
                .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to commit transaction: {}", e) })?;

            Ok(Some(Job {
                id,
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
                sns_video_id,
                published_at,
                output_videos,
            }))
        } else {
            Ok(None)
        }
    }

    async fn complete_job(&self, job_id: &str, output_videos: Option<&str>) -> Result<(), FactoryError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE jobs SET status = ?, output_videos = ?, updated_at = ? WHERE id = ?")
            .bind(JobStatus::Completed.to_string())
            .bind(output_videos)
            .bind(&now)
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to complete job {}: {}", job_id, e) })?;
        Ok(())
    }

    async fn fail_job(&self, job_id: &str, reason: &str) -> Result<(), FactoryError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE jobs SET status = ?, error_message = ?, updated_at = ? WHERE id = ?")
            .bind(JobStatus::Failed.to_string())
            .bind(reason)
            .bind(&now)
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to fail job {}: {}", job_id, e) })?;
        Ok(())
    }

    async fn fetch_relevant_karma(&self, topic: &str, skill_id: &str, limit: i64, current_soul_hash: &str) -> Result<Vec<String>, FactoryError> {
        // --- Phase 1: Boltzmann SQL Candidate Search ---
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
        .map_err(|e| FactoryError::Infrastructure { reason: format!("SQL Karma Query failed: {}", e) })?;

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

        // --- Phase 2: Semantic Re-Ranking (Optimized RAG) ---
        if let Some(ref api_key) = self.gemini_api_key {
            if let Ok(client) = gemini::Client::new(api_key) {
                // Only embed current target topic (Candidates are already embedded in DB)
                if let Ok(topic_builder) = client.embeddings::<String>("text-embedding-004").document(topic.to_string()) {
                    if let Ok(topic_res) = topic_builder.build().await {
                        if let Some((_, topic_many)) = topic_res.first() {
                            let topic_vec = &topic_many.first().vec;

                            for candidate in candidates.iter_mut() {
                                if let Some(ref emb_vec) = candidate.stored_embedding {
                                    candidate.semantic_score = cosine_similarity(topic_vec, emb_vec);
                                }
                            }

                            // Re-rank by Semantic Score (70%) + SQL Weight (30%)
                            candidates.sort_by(|a, b| {
                                let score_a = a.semantic_score * 0.7 + (a.sql_weight / 100.0) * 0.3;
                                let score_b = b.semantic_score * 0.7 + (b.sql_weight / 100.0) * 0.3;
                                score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
                            });
                        }
                    } else {
                        warn!("🧬 [KarmaRAG] Failed to embed topic for semantic ranking. Falling back to SQL weight.");
                    }
                }
            }
        }
        // --- Phase 3: Selection & Transgenerational Warning ---
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

        // Usage tracking (Audit Log)
        let now = Utc::now().to_rfc3339();
        for r in &rows {
            let id: String = r.get("id");
            let _ = sqlx::query("UPDATE karma_logs SET last_applied_at = ? WHERE id = ?").bind(&now).bind(id).execute(&self.pool).await;
        }

        Ok(final_karma)
    }

    async fn store_karma(&self, job_id: &str, skill_id: &str, lesson: &str, karma_type: &str, soul_hash: &str) -> Result<(), FactoryError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let mut embedding: Option<Vec<u8>> = None;
        if let Some(ref api_key) = self.gemini_api_key {
            if let Ok(client) = gemini::Client::new(api_key) {
                if let Ok(builder) = client.embeddings::<String>("text-embedding-004").document(lesson.to_string()) {
                    if let Ok(res) = builder.build().await {
                        if let Some((_, many)) = res.first() {
                            let emb = many.first();
                            let bytes: Vec<u8> = emb.vec.iter().flat_map(|f| f.to_le_bytes()).collect();
                            embedding = Some(bytes);
                        }
                    } else {
                        warn!("🧬 [KarmaStore] Failed to generate embedding for lesson (ignoring)");
                    }
                }
            }
        }

        sqlx::query(
            "INSERT INTO karma_logs (id, job_id, karma_type, related_skill, lesson, soul_version_hash, created_at, karma_embedding) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(job_id)
        .bind(karma_type)
        .bind(skill_id)
        .bind(lesson)
        .bind(soul_hash)
        .bind(&now)
        .bind(embedding)
        .execute(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to store karma for job {}: {}", job_id, e) })?;
        Ok(())
    }

    /// The Zombie Hunter (Heartbeat Edition): Reclaims jobs whose heartbeat has gone silent.
    /// Uses `last_heartbeat` instead of `started_at`, preventing false kills on long-running jobs.
    async fn reclaim_zombie_jobs(&self, timeout_minutes: i64) -> Result<u64, FactoryError> {
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
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to reclaim zombie jobs: {}", e) })?;

        let count = result.rows_affected();
        if count > 0 {
            tracing::warn!("🧟 Zombie Hunter: Reclaimed {} ghost job(s)", count);
        }
        Ok(count)
    }

    /// Sets the creative rating for a completed job (Human-in-the-Loop, Asynchronous Karma).
    /// Atomic Guard: Only Completed or Processing jobs can receive ratings.
    async fn set_creative_rating(&self, job_id: &str, rating: i32) -> Result<(), FactoryError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE jobs SET creative_rating = ?, updated_at = ? WHERE id = ? AND status IN ('Completed', 'Processing')"
        )
        .bind(rating)
        .bind(&now)
        .bind(job_id)
        .execute(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to set creative rating for job {}: {}", job_id, e) })?;

        if result.rows_affected() == 0 {
            return Err(FactoryError::Infrastructure {
                reason: format!("Atomic Guard: Job '{}' is not in Completed/Processing state, rating rejected", job_id),
            });
        }
        Ok(())
    }

    /// The Heartbeat Pulse: Worker calls this periodically to prove it's alive.
    async fn heartbeat_pulse(&self, job_id: &str) -> Result<(), FactoryError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE jobs SET last_heartbeat = ?, updated_at = ? WHERE id = ?")
            .bind(&now)
            .bind(&now)
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to pulse heartbeat for job {}: {}", job_id, e) })?;
        Ok(())
    }

    /// Log-First Distillation: Stores the execution log in the DB.
    async fn store_execution_log(&self, job_id: &str, log: &str) -> Result<(), FactoryError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE jobs SET execution_log = ?, updated_at = ? WHERE id = ?")
            .bind(log)
            .bind(&now)
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to store execution log for job {}: {}", job_id, e) })?;
        Ok(())
    }

    /// Deferred Distillation: Find completed/failed jobs with logs but no karma extracted yet.
    async fn fetch_undistilled_jobs(&self, limit: i64) -> Result<Vec<Job>, FactoryError> {
        let rows = sqlx::query(
            "SELECT id, topic, style_name, karma_directives, status, started_at, last_heartbeat, 
                     tech_karma_extracted, creative_rating, execution_log, error_message,
                     sns_platform, sns_video_id, published_at, output_videos 
              FROM jobs 
              WHERE execution_log IS NOT NULL 
              AND tech_karma_extracted = 0 
              AND status IN ('Completed', 'Failed') 
              ORDER BY updated_at ASC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to fetch undistilled jobs: {}", e) })?;

        let mut jobs = Vec::new();
        for r in rows {
            let tech_karma_extracted: i32 = r.get("tech_karma_extracted");
            jobs.push(Job {
                id: r.get("id"),
                topic: r.get("topic"),
                style: r.get("style_name"),
                karma_directives: try_get_optional_string(&r, "karma_directives"),
                status: match r.get::<String, _>("status").as_str() {
                    "Completed" => JobStatus::Completed,
                    "Failed" => JobStatus::Failed,
                    _ => JobStatus::Pending,
                },
                started_at: try_get_optional_string(&r, "started_at"),
                last_heartbeat: try_get_optional_string(&r, "last_heartbeat"),
                tech_karma_extracted: tech_karma_extracted != 0,
                creative_rating: r.try_get("creative_rating").ok(),
                execution_log: try_get_optional_string(&r, "execution_log"),
                error_message: try_get_optional_string(&r, "error_message"),
                sns_platform: try_get_optional_string(&r, "sns_platform"),
                sns_video_id: try_get_optional_string(&r, "sns_video_id"),
                published_at: try_get_optional_string(&r, "published_at"),
                output_videos: try_get_optional_string(&r, "output_videos"),
            });
        }
        Ok(jobs)
    }

    /// Marks a job as having had its karma extracted (tech_karma_extracted = 1).
    async fn mark_karma_extracted(&self, job_id: &str) -> Result<(), FactoryError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE jobs SET tech_karma_extracted = 1, updated_at = ? WHERE id = ?")
            .bind(&now)
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to mark karma extracted for job {}: {}", job_id, e) })?;
        Ok(())
    }

    /// DB Scavenger: Purge Completed/Failed jobs older than `days` days.
    /// karma_logs survive via ON DELETE SET NULL (Eternal Karma — jobs die, lessons live).
    /// Rigid Review: Purge threshold is typically >30 days (e.g. 60) to prevent the Watcher from losing targets.
    async fn purge_old_jobs(&self, days: i64) -> Result<u64, FactoryError> {
        let result = sqlx::query(
            "DELETE FROM jobs WHERE status IN ('Completed', 'Failed') AND created_at < datetime('now', ? || ' days')"
        )
        .bind(format!("-{}", days))
        .execute(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to purge old jobs: {}", e) })?;

        let purged = result.rows_affected();

        // Optimize DB after purge (lightweight alternative to VACUUM for WAL mode)
        let _ = sqlx::query("PRAGMA optimize;").execute(&self.pool).await;

        Ok(purged)
    }

    async fn link_sns_data(&self, job_id: &str, platform: &str, video_id: &str) -> Result<(), FactoryError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE jobs SET sns_platform = ?, sns_video_id = ?, published_at = ?, updated_at = ? WHERE id = ?")
            .bind(platform)
            .bind(video_id)
            .bind(&now)
            .bind(&now)
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to link SNS data for job {}: {}", job_id, e) })?;
        Ok(())
    }

    async fn fetch_jobs_for_evaluation(&self, milestone_days: i64, limit: i64) -> Result<Vec<Job>, FactoryError> {
        // The Catch-up Logic: State-based query that finds jobs past their milestone without a record.
        let rows = sqlx::query(
            "SELECT id, topic, style_name, karma_directives, status, started_at, last_heartbeat, 
                     tech_karma_extracted, creative_rating, execution_log, error_message,
                     sns_platform, sns_video_id, published_at, output_videos 
              FROM jobs 
              WHERE sns_platform IS NOT NULL 
              AND sns_video_id IS NOT NULL 
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
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to fetch jobs for evaluation: {}", e) })?;

        let mut jobs = Vec::new();
        for r in rows {
            let tech_karma_extracted: i32 = r.get("tech_karma_extracted");
            jobs.push(Job {
                id: r.get("id"),
                topic: r.get("topic"),
                style: r.get("style_name"),
                karma_directives: try_get_optional_string(&r, "karma_directives"),
                status: match r.get::<String, _>("status").as_str() {
                    "Completed" => JobStatus::Completed,
                    "Failed" => JobStatus::Failed,
                    _ => JobStatus::Pending,
                },
                started_at: try_get_optional_string(&r, "started_at"),
                last_heartbeat: try_get_optional_string(&r, "last_heartbeat"),
                tech_karma_extracted: tech_karma_extracted != 0,
                creative_rating: r.try_get("creative_rating").ok(),
                execution_log: try_get_optional_string(&r, "execution_log"),
                error_message: try_get_optional_string(&r, "error_message"),
                sns_platform: try_get_optional_string(&r, "sns_platform"),
                sns_video_id: try_get_optional_string(&r, "sns_video_id"),
                published_at: try_get_optional_string(&r, "published_at"),
                output_videos: try_get_optional_string(&r, "output_videos"),
            });
        }
        Ok(jobs)
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
    ) -> Result<(), FactoryError> {
        // --- #11 Statistical Pre-processing (Hard Metrics) ---
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
            "INSERT INTO sns_metrics_history (job_id, milestone_days, views, likes, comments_count, raw_comments_json, hard_metric_score, engagement_rate)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(job_id)
        .bind(milestone_days)
        .bind(views)
        .bind(likes)
        .bind(comments_count)
        .bind(raw_comments)
        .bind(hard_metric_score)
        .bind(engagement_rate)
        .execute(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to record SNS metrics: {}", e) })?;
        Ok(())
    }
    async fn fetch_pending_evaluations(&self, limit: i64) -> Result<Vec<SnsMetricsRecord>, FactoryError> {
        let rows = sqlx::query(
            "SELECT id, job_id, milestone_days, views, likes, comments_count, raw_comments_json, hard_metric_score, engagement_rate
             FROM sns_metrics_history
             WHERE is_finalized = 0
             LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to fetch pending evaluations: {}", e) })?;

        let mut out = Vec::new();
        for row in rows {
            out.push(SnsMetricsRecord {
                id: row.get("id"),
                job_id: row.get("job_id"),
                milestone_days: row.get("milestone_days"),
                views: row.get("views"),
                likes: row.get("likes"),
                comments_count: row.get("comments_count"),
                raw_comments_json: row.get("raw_comments_json"),
                hard_metric_score: row.try_get("hard_metric_score").ok(),
                engagement_rate: row.try_get("engagement_rate").ok(),
            });
        }
        Ok(out)
    }

    async fn apply_final_verdict(
        &self,
        record_id: i64,
        verdict: OracleVerdict,
        soul_hash: &str,
    ) -> Result<(), FactoryError> {
        let mut tx = self.pool.begin().await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to start transaction: {}", e) })?;

        // 1. Update the Metrics Ledger (The Proof)
        sqlx::query(
            "UPDATE sns_metrics_history 
             SET oracle_score_topic = ?, oracle_score_visual = ?, oracle_score_soul = ?, oracle_reason = ?, is_finalized = 1
             WHERE id = ?"
        )
        .bind(verdict.topic_score)
        .bind(verdict.visual_score)
        .bind(verdict.soul_score)
        .bind(&verdict.reasoning)
        .bind(record_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to update ledger: {}", e) })?;

        // 2. Fetch job info for Karma update
        let job_row = sqlx::query(
            "SELECT j.id, j.topic, j.style_name, h.milestone_days 
             FROM jobs j 
             JOIN sns_metrics_history h ON j.id = h.job_id 
             WHERE h.id = ?"
        )
        .bind(record_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to fetch job context: {}", e) })?;

        let job_id: String = job_row.get("id");
        let topic: String = job_row.get("topic");
        let style_name: String = job_row.get("style_name");
        let milestone_days: i64 = job_row.get("milestone_days");

        // 3. If it's the Final Verdict (30d), store the lesson in Karma Logs
        // Average Engagement * Soul Score => Weight (0-100)
        if milestone_days == 30 {
            // Semantic Karma Refinement (The Semantic Void 修正)
            // もし魂が汚染されていたら、Oracleの理由を「新たな戒め」として最高優先度で叩き込む
            if verdict.soul_score <= 0.5 {
                let karma_id = Uuid::new_v4().to_string();
                let lesson = format!("SOUL VIOLATION / 魂の汚染: {}", verdict.reasoning);
                
                sqlx::query(
                    "INSERT INTO karma_logs (id, job_id, karma_type, related_skill, lesson, weight, soul_version_hash)
                     VALUES (?, ?, ?, ?, ?, ?, ?)"
                )
                .bind(&karma_id)
                .bind(&job_id)
                .bind("Synthesized") // 新たな叡智・戒めとして合成
                .bind(&style_name) // ここでの関連スキルは映像スタイル
                .bind(&lesson)
                .bind(100) // 絶対的な掟として RAG のトップに固定
                .bind(soul_hash)
                .execute(&mut *tx)
                .await
                .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to inject Semantic Refinement: {}", e) })?;
            }
            let avg_engagement = (verdict.topic_score + verdict.visual_score) / 2.0;
            let calculated_weight = (50.0 + (avg_engagement * verdict.soul_score * 50.0)) as i64;
            let weight = calculated_weight.clamp(0, 100);

            sqlx::query(
                "INSERT INTO karma_logs (job_id, topic, style_name, lesson, weight, soul_version_hash)
                 VALUES (?, ?, ?, ?, ?, ?)"
            )
            .bind(&job_id)
            .bind(&topic)
            .bind(&style_name)
            .bind(&verdict.reasoning)
            .bind(weight)
            .bind(soul_hash)
            .execute(&mut *tx)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to update Karma logs: {}", e) })?;
        }

        tx.commit().await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to commit transaction: {}", e) })?;

        Ok(())
    }

    async fn fetch_recent_jobs(&self, limit: i64) -> Result<Vec<Job>, FactoryError> {
        let rows = sqlx::query(
            "SELECT id, topic, style_name, karma_directives, status, started_at, last_heartbeat, 
                     tech_karma_extracted, creative_rating, execution_log, error_message,
                     sns_platform, sns_video_id, published_at, output_videos 
              FROM jobs 
              ORDER BY created_at DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to fetch recent jobs: {}", e) })?;

        let mut jobs = Vec::new();
        for r in rows {
            let tech_karma_extracted: i32 = r.get("tech_karma_extracted");
            jobs.push(Job {
                id: r.get("id"),
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
                sns_video_id: try_get_optional_string(&r, "sns_video_id"),
                published_at: try_get_optional_string(&r, "published_at"),
                output_videos: try_get_optional_string(&r, "output_videos"),
            });
        }
        Ok(jobs)
    }

    async fn get_agent_stats(&self) -> Result<shared::watchtower::AgentStats, FactoryError> {
        let row = sqlx::query("SELECT level, exp, affection, intimacy, fatigue FROM agent_stats WHERE id = 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to fetch agent stats: {}", e) })?;

        use sqlx::Row;
        Ok(shared::watchtower::AgentStats {
            level: row.get("level"),
            exp: row.get("exp"),
            affection: row.get("affection"),
            intimacy: row.get("intimacy"),
            fatigue: row.get("fatigue"),
        })
    }

    async fn add_affection(&self, amount: i32) -> Result<(), FactoryError> {
        sqlx::query("UPDATE agent_stats SET affection = affection + ?, updated_at = datetime('now') WHERE id = 1")
            .bind(amount)
            .execute(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to update affection: {}", e) })?;
        Ok(())
    }

    async fn add_tech_exp(&self, amount: i32) -> Result<(), FactoryError> {
        sqlx::query("UPDATE agent_stats SET exp = exp + ?, updated_at = datetime('now') WHERE id = 1")
            .bind(amount)
            .execute(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to update exp: {}", e) })?;
        Ok(())
    }

    async fn add_intimacy(&self, amount: i32) -> Result<(), FactoryError> {
        sqlx::query("UPDATE agent_stats SET intimacy = intimacy + ?, updated_at = datetime('now') WHERE id = 1")
            .bind(amount)
            .execute(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to update intimacy: {}", e) })?;
        Ok(())
    }

    async fn get_pending_job_count(&self) -> Result<i64, FactoryError> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM jobs WHERE status = ?")
            .bind(JobStatus::Pending.to_string())
            .fetch_one(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to count pending jobs: {}", e) })?;
        Ok(row.get("count"))
    }

    async fn get_job_count_since(&self, since: chrono::DateTime<chrono::Utc>) -> Result<i64, FactoryError> {
        let since_str = since.to_rfc3339();
        let row = sqlx::query("SELECT COUNT(*) as count FROM jobs WHERE created_at >= ?")
            .bind(since_str)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to count jobs since: {}", e) })?;
        Ok(row.get("count"))
    }

    async fn fetch_all_karma(&self, limit: i64) -> Result<Vec<serde_json::Value>, FactoryError> {
        let rows = sqlx::query(
            "SELECT * FROM karma_logs ORDER BY created_at DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: e.to_string() })?;

        let mut karmas = Vec::new();
        for row in rows {
            use sqlx::Row;
            karmas.push(serde_json::json!({
                "id": row.try_get::<String, _>("id").unwrap_or_default(),
                "job_id": row.try_get::<String, _>("job_id").unwrap_or_default(),
                "skill_id": row.try_get::<String, _>("related_skill").unwrap_or_default(),
                "lesson": row.try_get::<String, _>("lesson").unwrap_or_default(),
                "karma_type": row.try_get::<String, _>("karma_type").unwrap_or_default(),
                "weight": row.try_get::<i64, _>("weight").unwrap_or_default(),
                "created_at": row.try_get::<String, _>("created_at").unwrap_or_default(),
                "last_applied_at": row.try_get::<Option<String>, _>("last_applied_at").unwrap_or_default(),
                "soul_version_hash": row.try_get::<Option<String>, _>("soul_version_hash").unwrap_or_default(),
            }));
        }
        Ok(karmas)
    }

    async fn fetch_top_performing_jobs(&self, limit: i64) -> Result<Vec<Job>, FactoryError> {
        let rows = sqlx::query(
            "SELECT j.* FROM jobs j 
             JOIN sns_metrics_history s ON j.id = s.job_id 
             WHERE s.is_finalized = 1 
             ORDER BY s.views DESC 
             LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: e.to_string() })?;

        let mut jobs = Vec::new();
        for r in rows {
            let id: String = r.get("id");
            let topic: String = r.get("topic");
            let style: String = r.get("style_name");
            let karma_directives: Option<String> = try_get_optional_string(&r, "karma_directives");
            let tech_karma_extracted: i32 = r.get("tech_karma_extracted");
            let creative_rating: Option<i32> = r.try_get("creative_rating").ok();
            let execution_log: Option<String> = try_get_optional_string(&r, "execution_log");
            let error_message: Option<String> = try_get_optional_string(&r, "error_message");
            let sns_platform: Option<String> = try_get_optional_string(&r, "sns_platform");
            let sns_video_id: Option<String> = try_get_optional_string(&r, "sns_video_id");
            let published_at: Option<String> = try_get_optional_string(&r, "published_at");
            let output_videos: Option<String> = try_get_optional_string(&r, "output_videos");
            let status_str: String = r.get("status");
            let status = JobStatus::from_string(&status_str);

            jobs.push(Job {
                id,
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
                sns_video_id,
                published_at,
                output_videos,
            });
        }
        Ok(jobs)
    }

    async fn record_soul_mutation(&self, old_hash: &str, new_hash: &str, reason: &str) -> Result<(), FactoryError> {
        sqlx::query("INSERT INTO soul_mutation_history (old_hash, new_hash, mutation_reason) VALUES (?, ?, ?)")
            .bind(old_hash)
            .bind(new_hash)
            .bind(reason)
            .execute(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: e.to_string() })?;
        Ok(())
    }

    async fn fetch_job_retry_count(&self, job_id: &str) -> Result<i64, FactoryError> {
        let row = sqlx::query("SELECT retry_count FROM jobs WHERE id = ?")
            .bind(job_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to fetch retry count: {}", e) })?;
        
        if let Some(r) = row {
            Ok(r.get("retry_count"))
        } else {
            Ok(0)
        }
    }

    async fn reset_job_retry_count(&self, job_id: &str) -> Result<(), FactoryError> {
        sqlx::query("UPDATE jobs SET retry_count = 0 WHERE id = ?")
            .bind(job_id)
            .execute(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to reset retry count: {}", e) })?;
        Ok(())
    }

    async fn increment_job_retry_count(&self, job_id: &str) -> Result<bool, FactoryError> {
        let row = sqlx::query("UPDATE jobs SET retry_count = retry_count + 1 WHERE id = ? RETURNING retry_count")
            .bind(job_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to increment job retry count: {}", e) })?;
            
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

    async fn fetch_unincorporated_karma(&self, limit: i64, current_soul_hash: &str) -> Result<Vec<serde_json::Value>, FactoryError> {
        let rows = sqlx::query(
            "SELECT id, lesson, related_skill, karma_type, weight FROM karma_logs 
             WHERE soul_version_hash IS NULL OR soul_version_hash != ? 
             ORDER BY created_at DESC LIMIT ?"
        )
        .bind(current_soul_hash)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to fetch unincorporated karma: {}", e) })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(serde_json::json!({
                "id": row.get::<String, _>("id"),
                "lesson": row.get::<String, _>("lesson"),
                "skill": row.get::<String, _>("related_skill"),
                "type": row.get::<String, _>("karma_type"),
                "weight": row.get::<i64, _>("weight"),
            }));
        }
        Ok(results)
    }

    async fn mark_karma_as_incorporated(&self, karma_ids: Vec<String>, new_soul_hash: &str) -> Result<(), FactoryError> {
        if karma_ids.is_empty() { return Ok(()); }
        
        // SQLite supports `IN (...)` with many parameters, but here we build it manually or use QueryBuilder
        let mut query_builder = sqlx::QueryBuilder::new("UPDATE karma_logs SET soul_version_hash = ");
        query_builder.push_bind(new_soul_hash);
        query_builder.push(", last_applied_at = datetime('now') WHERE id IN ( ");
        
        let mut separated = query_builder.separated(", ");
        for id in karma_ids {
            separated.push_bind(id);
        }
        query_builder.push(" )");
        
        query_builder.build()
            .execute(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to mark karma as incorporated: {}", e) })?;
        
        Ok(())
    }

    async fn store_immune_rule(&self, rule: &factory_core::contracts::ImmuneRule) -> Result<(), FactoryError> {
        sqlx::query("INSERT INTO immune_rules (id, pattern, severity, action, created_at) VALUES (?, ?, ?, ?, ?) ON CONFLICT(id) DO NOTHING")
            .bind(&rule.id)
            .bind(&rule.pattern)
            .bind(rule.severity as i64)
            .bind(&rule.action)
            .bind(&rule.created_at)
            .execute(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to store immune rule: {}", e) })?;
        Ok(())
    }

    async fn fetch_active_immune_rules(&self) -> Result<Vec<factory_core::contracts::ImmuneRule>, FactoryError> {
        let rows = sqlx::query("SELECT id, pattern, severity, action, created_at FROM immune_rules ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to fetch immune rules: {}", e) })?;

        let mut rules = Vec::new();
        for r in rows {
            rules.push(factory_core::contracts::ImmuneRule {
                id: r.try_get("id").unwrap_or_else(|_| "".to_string()),
                pattern: r.try_get("pattern").unwrap_or_else(|_| "".to_string()),
                severity: r.try_get::<i64, _>("severity").unwrap_or(50) as u8,
                action: r.try_get("action").unwrap_or_else(|_| "Block".to_string()),
                created_at: r.try_get("created_at").unwrap_or_else(|_| "".to_string()),
            });
        }
        Ok(rules)
    }

    async fn record_arena_match(&self, match_data: &factory_core::contracts::ArenaMatch) -> Result<(), FactoryError> {
        sqlx::query("INSERT INTO arena_history (id, skill_a, skill_b, topic, winner, reasoning, created_at) VALUES (?, ?, ?, ?, ?, ?, ?) ON CONFLICT(id) DO NOTHING")
            .bind(&match_data.id)
            .bind(&match_data.skill_a)
            .bind(&match_data.skill_b)
            .bind(&match_data.topic)
            .bind(&match_data.winner)
            .bind(&match_data.reasoning)
            .bind(&match_data.created_at)
            .execute(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to record arena match: {}", e) })?;
        Ok(())
    }

    // --- Phase 12-F: Karma Federation ---

    async fn export_federated_data(&self, since: Option<&str>) -> Result<(Vec<factory_core::contracts::FederatedKarma>, Vec<factory_core::contracts::ImmuneRule>, Vec<factory_core::contracts::ArenaMatch>), FactoryError> {
        let since_ts = since.unwrap_or("1970-01-01T00:00:00");

        let karmas = sqlx::query("SELECT id, job_id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at FROM karma_logs WHERE created_at > ?")
            .bind(since_ts)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Export Karma failed: {}", e) })?;

        let rules = sqlx::query("SELECT id, pattern, severity, action, created_at FROM immune_rules WHERE created_at > ?")
            .bind(since_ts)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Export Rules failed: {}", e) })?;

        let matches = sqlx::query("SELECT id, skill_a, skill_b, topic, winner, reasoning, created_at FROM arena_history WHERE created_at > ?")
            .bind(since_ts)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Export Matches failed: {}", e) })?;

        let mut fed_karmas = Vec::new();
        for r in karmas {
            fed_karmas.push(factory_core::contracts::FederatedKarma {
                id: r.get("id"),
                job_id: try_get_optional_string(&r, "job_id"),
                karma_type: r.get("karma_type"),
                related_skill: r.get("related_skill"),
                lesson: r.get("lesson"),
                weight: r.get::<i64, _>("weight") as i32,
                soul_version_hash: try_get_optional_string(&r, "soul_version_hash"),
                created_at: r.get("created_at"),
            });
        }

        let mut fed_rules = Vec::new();
        for r in rules {
            fed_rules.push(factory_core::contracts::ImmuneRule {
                id: r.get("id"),
                pattern: r.get("pattern"),
                severity: r.get::<i64, _>("severity") as u8,
                action: r.get("action"),
                created_at: r.get("created_at"),
            });
        }

        let mut fed_matches = Vec::new();
        for r in matches {
            fed_matches.push(factory_core::contracts::ArenaMatch {
                id: r.get("id"),
                skill_a: r.get("skill_a"),
                skill_b: r.get("skill_b"),
                topic: r.get("topic"),
                winner: try_get_optional_string(&r, "winner"),
                reasoning: r.get("reasoning"),
                created_at: r.get("created_at"),
            });
        }

        Ok((fed_karmas, fed_rules, fed_matches))
    }

    async fn import_federated_data(&self, karmas: Vec<factory_core::contracts::FederatedKarma>, rules: Vec<factory_core::contracts::ImmuneRule>, matches: Vec<factory_core::contracts::ArenaMatch>) -> Result<(), FactoryError> {
        let mut tx = self.pool.begin().await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Import Tx start failed: {}", e) })?;

        for k in karmas {
            sqlx::query("INSERT INTO karma_logs (id, job_id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(id) DO NOTHING")
                .bind(&k.id).bind(&k.job_id).bind(&k.karma_type).bind(&k.related_skill).bind(&k.lesson).bind(k.weight as i64).bind(&k.soul_version_hash).bind(&k.created_at)
                .execute(&mut *tx).await.map_err(|e| FactoryError::Infrastructure { reason: format!("Import Karma failed: {}", e) })?;
        }

        for r in rules {
            sqlx::query("INSERT INTO immune_rules (id, pattern, severity, action, created_at) VALUES (?, ?, ?, ?, ?) ON CONFLICT(id) DO NOTHING")
                .bind(&r.id).bind(&r.pattern).bind(r.severity as i64).bind(&r.action).bind(&r.created_at)
                .execute(&mut *tx).await.map_err(|e| FactoryError::Infrastructure { reason: format!("Import Rule failed: {}", e) })?;
        }

        for m in matches {
            sqlx::query("INSERT INTO arena_history (id, skill_a, skill_b, topic, winner, reasoning, created_at) VALUES (?, ?, ?, ?, ?, ?, ?) ON CONFLICT(id) DO NOTHING")
                .bind(&m.id).bind(&m.skill_a).bind(&m.skill_b).bind(&m.topic).bind(&m.winner).bind(&m.reasoning).bind(&m.created_at)
                .execute(&mut *tx).await.map_err(|e| FactoryError::Infrastructure { reason: format!("Import Match failed: {}", e) })?;
        }

        tx.commit().await.map_err(|e| FactoryError::Infrastructure { reason: format!("Import Tx commit failed: {}", e) })?;
        Ok(())
    }

    async fn get_peer_sync_time(&self, peer_url: &str) -> Result<Option<String>, FactoryError> {
        let row = sqlx::query("SELECT last_sync_at FROM federation_peers WHERE peer_url = ?")
            .bind(peer_url)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Get Peer sync time failed: {}", e) })?;
        
        Ok(row.map(|r| r.get("last_sync_at")))
    }

    async fn update_peer_sync_time(&self, peer_url: &str, sync_time: &str) -> Result<(), FactoryError> {
        sqlx::query("INSERT INTO federation_peers (peer_url, last_sync_at) VALUES (?, ?) ON CONFLICT(peer_url) DO UPDATE SET last_sync_at = excluded.last_sync_at")
            .bind(peer_url)
            .bind(sync_time)
            .execute(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Update Peer sync time failed: {}", e) })?;
        Ok(())
    }
}

impl SqliteJobQueue {
    // --- Watchtower Memory Distillation Methods ---

    pub async fn insert_chat_message(&self, channel_id: &str, role: &str, content: &str) -> Result<(), FactoryError> {
        sqlx::query("INSERT INTO chat_history (channel_id, role, content) VALUES (?, ?, ?)")
            .bind(channel_id)
            .bind(role)
            .bind(content)
            .execute(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to insert chat history: {}", e) })?;
        Ok(())
    }

    pub async fn fetch_chat_history(&self, channel_id: &str, limit: i64) -> Result<Vec<serde_json::Value>, FactoryError> {
        // Fetch the newest `limit` messages, but we need them in chronological order
        // So we order by id DESC, limit, and then reverse the result in memory.
        let rows = sqlx::query(
            "SELECT role, content FROM chat_history WHERE channel_id = ? ORDER BY id DESC LIMIT ?"
        )
        .bind(channel_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to fetch chat history: {}", e) })?;

        let mut messages = Vec::new();
        for row in rows {
            use sqlx::Row;
            let role: String = row.get("role");
            let content: String = row.get("content");
            messages.push(serde_json::json!({
                "role": role,
                "content": content
            }));
        }
        
        // Output needs to be chronological (oldest first)
        messages.reverse();
        Ok(messages)
    }

    pub async fn get_chat_memory_summary(&self, channel_id: &str) -> Result<Option<String>, FactoryError> {
        let row = sqlx::query("SELECT summary FROM chat_memory_summaries WHERE channel_id = ?")
            .bind(channel_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to get chat memory summary: {}", e) })?;

        if let Some(r) = row {
            use sqlx::Row;
            Ok(Some(r.get("summary")))
        } else {
            Ok(None)
        }
    }

    pub async fn update_chat_memory_summary(&self, channel_id: &str, summary: &str) -> Result<(), FactoryError> {
        sqlx::query(
            "INSERT INTO chat_memory_summaries (channel_id, summary, updated_at) 
             VALUES (?, ?, datetime('now'))
             ON CONFLICT(channel_id) DO UPDATE SET summary = excluded.summary, updated_at = excluded.updated_at"
        )
        .bind(channel_id)
        .bind(summary)
        .execute(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to update chat memory summary: {}", e) })?;
        Ok(())
    }

    /// Fetches all undistilled chats spanning all channels. 
    /// Returns a map of channel_id to a list of (id, role, content)
    pub async fn fetch_undistilled_chats_by_channel(&self) -> Result<std::collections::HashMap<String, Vec<(i64, String, String)>>, FactoryError> {
        let rows = sqlx::query(
            "SELECT id, channel_id, role, content FROM chat_history WHERE is_distilled = 0 ORDER BY channel_id ASC, id ASC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to fetch undistilled chats: {}", e) })?;

        let mut map = std::collections::HashMap::new();
        for row in rows {
            use sqlx::Row;
            let id: i64 = row.get("id");
            let channel_id: String = row.get("channel_id");
            let role: String = row.get("role");
            let content: String = row.get("content");
            map.entry(channel_id).or_insert_with(Vec::new).push((id, role, content));
        }
        Ok(map)
    }

    pub async fn mark_chats_as_distilled(&self, channel_id: &str, up_to_id: i64) -> Result<(), FactoryError> {
        sqlx::query("UPDATE chat_history SET is_distilled = 1 WHERE channel_id = ? AND id <= ?")
            .bind(channel_id)
            .bind(up_to_id)
            .execute(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to mark chats as distilled: {}", e) })?;
        Ok(())
    }

    pub async fn purge_old_distilled_chats(&self, days: i64) -> Result<u64, FactoryError> {
        let result = sqlx::query(
            "DELETE FROM chat_history WHERE is_distilled = 1 AND created_at < datetime('now', ? || ' days')"
        )
        .bind(format!("-{}", days))
        .execute(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to purge old distilled chats: {}", e) })?;

        Ok(result.rows_affected())
    }

    // --- Consolidated Inherent Methods ---
    pub async fn fetch_skills_for_distillation(&self, threshold: i64) -> Result<Vec<String>, FactoryError> {
        let rows = sqlx::query(
            "SELECT related_skill FROM karma_logs GROUP BY related_skill HAVING COUNT(id) > ?"
        )
        .bind(threshold)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to fetch skills for distillation: {}", e) })?;

        let mut skills = Vec::new();
        for r in rows {
            skills.push(r.try_get("related_skill").unwrap_or_else(|_| "".to_string()));
        }
        Ok(skills)
    }

    pub async fn fetch_raw_karma_for_skill(&self, skill: &str) -> Result<Vec<(String, String)>, FactoryError> {
        let rows = sqlx::query(
            "SELECT id, lesson FROM karma_logs WHERE related_skill = ?"
        )
        .bind(skill)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to fetch raw karma for skill: {}", e) })?;

        let mut karma = Vec::new();
        for r in rows {
            let id: String = try_get_optional_string(&r, "id").unwrap_or_else(|| "".to_string());
            let lesson: String = try_get_optional_string(&r, "lesson").unwrap_or_else(|| "".to_string());
            karma.push((id, lesson));
        }
        Ok(karma)
    }


    pub async fn apply_distilled_karma(&self, skill: &str, distilled_lesson: &str, old_karma_ids: &[String], soul_hash: &str) -> Result<(), FactoryError> {
        let mut tx = self.pool.begin().await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to start tx for distillation: {}", e) })?;

        for id in old_karma_ids {
            sqlx::query("DELETE FROM karma_logs WHERE id = ?").bind(id).execute(&mut *tx).await
                .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to delete old karma {}: {}", id, e) })?;
        }

        let new_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO karma_logs (id, karma_type, related_skill, lesson, weight, soul_version_hash)
             VALUES (?, 'Synthesized', ?, ?, 100, ?)"
        )
            .bind(&new_id)
            .bind(skill)
            .bind(distilled_lesson)
            .bind(soul_hash)
            .execute(&mut *tx)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to insert synthesized karma: {}", e) })?;

        tx.commit().await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to commit distlillation tx: {}", e) })?;

        Ok(())
    }



    pub async fn increment_oracle_retry_count(&self, record_id: i64) -> Result<bool, FactoryError> {
        let row = sqlx::query("UPDATE sns_metrics_history SET retry_count = retry_count + 1 WHERE id = ? RETURNING retry_count")
            .bind(record_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to increment oracle retry count: {}", e) })?;
            
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

    pub async fn get_global_api_failures(&self) -> Result<i64, FactoryError> {
        let row = sqlx::query("SELECT value FROM system_state WHERE key = 'consecutive_api_failures'")
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to read system_state: {}", e) })?;
        
        if let Some(r) = row {
            let val_str: String = r.try_get("value").unwrap_or_default();
            Ok(val_str.parse().unwrap_or(0))
        } else {
            Ok(0)
        }
    }

    pub async fn record_global_api_failure(&self) -> Result<i64, FactoryError> {
        let current = self.get_global_api_failures().await?;
        let next = current + 1;
        
        sqlx::query(
            "INSERT INTO system_state (key, value, updated_at) 
             VALUES ('consecutive_api_failures', ?, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at"
        )
        .bind(next.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to update system_state: {}", e) })?;
        
        Ok(next)
    }

    pub async fn record_global_api_success(&self) -> Result<(), FactoryError> {
        sqlx::query(
            "INSERT INTO system_state (key, value, updated_at) 
             VALUES ('consecutive_api_failures', '0', datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to reset system_state: {}", e) })?;
        
        Ok(())
    }

    pub async fn fetch_unfederated_data(&self) -> Result<(Vec<factory_core::contracts::FederatedKarma>, Vec<factory_core::contracts::ImmuneRule>), FactoryError> {
        let karmas = sqlx::query("SELECT id, job_id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at FROM karma_logs WHERE is_federated = 0")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Fetch unfederated karma failed: {}", e) })?;

        let rules = sqlx::query("SELECT id, pattern, severity, action, created_at FROM immune_rules WHERE is_federated = 0")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Fetch unfederated rules failed: {}", e) })?;

        let mut fed_karmas = Vec::new();
        for r in karmas {
            use sqlx::Row;
            fed_karmas.push(factory_core::contracts::FederatedKarma {
                id: r.get("id"),
                job_id: try_get_optional_string(&r, "job_id"),
                karma_type: r.get("karma_type"),
                related_skill: r.get("related_skill"),
                lesson: r.get("lesson"),
                weight: r.get::<i64, _>("weight") as i32,
                soul_version_hash: try_get_optional_string(&r, "soul_version_hash"),
                created_at: r.get("created_at"),
            });
        }

        let mut fed_rules = Vec::new();
        for r in rules {
            use sqlx::Row;
            fed_rules.push(factory_core::contracts::ImmuneRule {
                id: r.get("id"),
                pattern: r.get("pattern"),
                severity: r.get::<i64, _>("severity") as u8,
                action: r.get("action"),
                created_at: r.get("created_at"),
            });
        }

        Ok((fed_karmas, fed_rules))
    }

    pub async fn mark_as_federated(&self, karma_ids: Vec<String>, rule_ids: Vec<String>) -> Result<(), FactoryError> {
        let mut tx = self.pool.begin().await
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Mark federated Tx failed: {}", e) })?;

        for id in karma_ids {
            sqlx::query("UPDATE karma_logs SET is_federated = 1 WHERE id = ?").bind(id).execute(&mut *tx).await.ok();
        }
        for id in rule_ids {
            sqlx::query("UPDATE immune_rules SET is_federated = 1 WHERE id = ?").bind(id).execute(&mut *tx).await.ok();
        }

        tx.commit().await.map_err(|e| FactoryError::Infrastructure { reason: format!("Mark federated commit failed: {}", e) })?;
        Ok(())
    }
}


// Helper function because `get` on Option panics if type is unexpected, 
// using try_get is safer if column can be NULL.
fn try_get_optional_string(row: &sqlx::sqlite::SqliteRow, col: &str) -> Option<String> {
    use sqlx::Row;
    row.try_get(col).ok()
}

fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let dot_product: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 { return 0.0; }
    dot_product / (norm_a * norm_b)
}
