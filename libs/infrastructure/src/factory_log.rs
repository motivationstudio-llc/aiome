/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

//! # FactoryLog — 実行ログ・監査証跡システム
//!
//! SQLite を使用して動画生成の履歴、エラー、およびセキュリティイベント（ブロック記録）を保存する。

use async_trait::async_trait;
use factory_core::error::FactoryError;
use factory_core::traits::FactoryLogger;
use sqlx::sqlite::SqlitePool;
use std::path::PathBuf;

/// SQLite をバックエンドとするロガークライアント
pub struct FactoryLogClient {
    pub db: SqlitePool,
}

impl FactoryLogClient {
    pub async fn new(db_path: &str) -> Result<Self, FactoryError> {
        let pool = SqlitePool::connect(&format!("sqlite:{}", db_path)).await.map_err(|e| {
            FactoryError::Infrastructure {
                reason: format!("Failed to connect to SQLite: {}", e),
            }
        })?;

        // テーブルの初期化
        sqlx::query::<sqlx::Sqlite>(
            "CREATE TABLE IF NOT EXISTS logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                event_type TEXT NOT NULL,
                video_id TEXT,
                output_path TEXT,
                detail TEXT
            )"
        )
        .execute(&pool)
        .await
        .map_err(|e| FactoryError::Infrastructure {
            reason: format!("Failed to initialize database: {}", e),
        })?;

        Ok(Self { db: pool })
    }
}

#[async_trait]
impl FactoryLogger for FactoryLogClient {
    async fn log_success(&self, video_id: &str, output_path: &PathBuf) -> Result<(), FactoryError> {
        sqlx::query::<sqlx::Sqlite>("INSERT INTO logs (event_type, video_id, output_path) VALUES (?, ?, ?)")
            .bind("SUCCESS")
            .bind(video_id)
            .bind(output_path.to_string_lossy().to_string())
            .execute(&self.db)
            .await
            .map_err(|e| FactoryError::Infrastructure {
                reason: format!("Log insertion failed: {}", e),
            })?;
        
        Ok(())
    }

    async fn log_error(&self, reason: &str) -> Result<(), FactoryError> {
        sqlx::query::<sqlx::Sqlite>("INSERT INTO logs (event_type, detail) VALUES (?, ?)")
            .bind("ERROR")
            .bind(reason)
            .execute(&self.db)
            .await
            .map_err(|e| FactoryError::Infrastructure {
                reason: format!("Log insertion failed: {}", e),
            })?;
        
        Ok(())
    }

    async fn daily_summary(&self, _jail: &bastion::fs_guard::Jail) -> Result<String, FactoryError> {
        // 簡易的なサマリー取得
        let count: (i64,) = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM logs WHERE event_type = 'SUCCESS'")
            .fetch_one(&self.db)
            .await
            .map_err(|e| FactoryError::Infrastructure {
                reason: format!("Summary failed: {}", e),
            })?;
        
        Ok(format!("本日の動画生成成功数: {} 本", count.0))
    }
}
