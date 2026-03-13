/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Elastic License 2.0 (ELv2).
 * You may not provide the software to third parties as a hosted or managed service,
 * where the service provides users with access to any substantial set of the features
 * or functionality of the software.
 */

//! # AiomeLog — 実行ログ・監査証跡システム
//!
//! SQLite を使用して成果物生成の履歴、エラー、およびセキュリティイベント（ブロック記録）を保存する。

use aiome_core::error::AiomeError;
use aiome_core::traits::AiomeLogger;
use async_trait::async_trait;
use sqlx::sqlite::SqlitePool;
use std::path::PathBuf;

use sha2::{Digest, Sha256};

/// SQLite をバックエンドとするロガークライアント
pub struct AiomeLogClient {
    pub db: SqlitePool,
}

impl AiomeLogClient {
    pub async fn new(db_path: &str) -> Result<Self, AiomeError> {
        let pool = SqlitePool::connect(&format!("sqlite:{}", db_path))
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to connect to SQLite: {}", e),
            })?;

        // テーブルの初期化
        sqlx::query::<sqlx::Sqlite>(
            "CREATE TABLE IF NOT EXISTS logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                event_type TEXT NOT NULL,
                artifact_id TEXT,
                output_path TEXT,
                detail TEXT,
                hash TEXT
            )",
        )
        .execute(&pool)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to initialize database: {}", e),
        })?;

        // マイグレーション: hash カラムが存在しない場合に備えて
        let _ = sqlx::query("ALTER TABLE logs ADD COLUMN hash TEXT")
            .execute(&pool)
            .await;

        Ok(Self { db: pool })
    }

    async fn get_last_hash(&self) -> String {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT hash FROM logs WHERE hash IS NOT NULL ORDER BY id DESC LIMIT 1")
                .fetch_optional(&self.db)
                .await
                .unwrap_or(None);

        row.map(|r| r.0)
            .unwrap_or_else(|| "AIOME_GENESIS_HASH_2026".to_string())
    }

    fn compute_hash(
        prev_hash: &str,
        event_type: &str,
        artifact: Option<&str>,
        path: Option<&str>,
        detail: Option<&str>,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(prev_hash.as_bytes());
        hasher.update(event_type.as_bytes());
        hasher.update(artifact.unwrap_or("").as_bytes());
        hasher.update(path.unwrap_or("").as_bytes());
        hasher.update(detail.unwrap_or("").as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[async_trait]
impl AiomeLogger for AiomeLogClient {
    async fn log_success(
        &self,
        artifact_id: &str,
        output_path: &PathBuf,
    ) -> Result<(), AiomeError> {
        let prev_hash = self.get_last_hash().await;
        let artifact = Some(artifact_id);
        let path_str = output_path.to_string_lossy();
        let path = Some(path_str.as_ref());
        let hash = Self::compute_hash(&prev_hash, "SUCCESS", artifact, path, None);

        sqlx::query::<sqlx::Sqlite>(
            "INSERT INTO logs (event_type, artifact_id, output_path, hash) VALUES (?, ?, ?, ?)",
        )
        .bind("SUCCESS")
        .bind(artifact_id)
        .bind(output_path.to_string_lossy().to_string())
        .bind(hash)
        .execute(&self.db)
        .await
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Log insertion failed: {}", e),
        })?;

        Ok(())
    }

    async fn log_error(&self, reason: &str) -> Result<(), AiomeError> {
        let prev_hash = self.get_last_hash().await;
        let detail = Some(reason);
        let hash = Self::compute_hash(&prev_hash, "ERROR", None, None, detail);

        sqlx::query::<sqlx::Sqlite>("INSERT INTO logs (event_type, detail, hash) VALUES (?, ?, ?)")
            .bind("ERROR")
            .bind(reason)
            .bind(hash)
            .execute(&self.db)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Log insertion failed: {}", e),
            })?;

        Ok(())
    }

    async fn daily_summary(&self, _jail: &bastion::fs_guard::Jail) -> Result<String, AiomeError> {
        // 簡易的なサマリー取得
        let count: (i64,) =
            sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM logs WHERE event_type = 'SUCCESS'")
                .fetch_one(&self.db)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Summary failed: {}", e),
                })?;

        Ok(format!("本日の成果物生成成功数: {} 本", count.0))
    }
}
