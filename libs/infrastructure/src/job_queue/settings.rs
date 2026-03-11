/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use async_trait::async_trait;
use crate::job_queue::SqliteJobQueue;
use aiome_core::error::AiomeError;
use sqlx::Row;

#[async_trait]
pub trait SettingsOps {
    async fn get_setting(&self, key: &str) -> Result<Option<String>, AiomeError>;
    async fn set_setting(&self, key: &str, value: &str, category: &str, is_secret: bool) -> Result<(), AiomeError>;
    async fn get_all_settings(&self) -> Result<Vec<SettingEntry>, AiomeError>;
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SettingEntry {
    pub key: String,
    pub value: String,
    pub category: String,
    pub is_secret: bool,
    pub updated_at: String,
}

#[async_trait]
impl SettingsOps for SqliteJobQueue {
    async fn get_setting(&self, key: &str) -> Result<Option<String>, AiomeError> {
        let row = sqlx::query("SELECT value FROM system_settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        Ok(row.map(|r| r.get(0)))
    }

    async fn set_setting(&self, key: &str, value: &str, category: &str, is_secret: bool) -> Result<(), AiomeError> {
        sqlx::query(
            "INSERT INTO system_settings (key, value, category, is_secret, updated_at) 
             VALUES (?, ?, ?, ?, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET 
                value = excluded.value, 
                category = excluded.category, 
                is_secret = excluded.is_secret,
                updated_at = datetime('now')"
        )
        .bind(key)
        .bind(value)
        .bind(category)
        .bind(is_secret as i32)
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        Ok(())
    }

    async fn get_all_settings(&self) -> Result<Vec<SettingEntry>, AiomeError> {
        let rows = sqlx::query("SELECT key, value, category, is_secret, updated_at FROM system_settings")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(SettingEntry {
                key: row.get("key"),
                value: row.get("value"),
                category: row.get("category"),
                is_secret: row.get::<i32, _>("is_secret") != 0,
                updated_at: row.get("updated_at"),
            });
        }
        Ok(entries)
    }
}
