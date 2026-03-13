/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use super::SqliteJobQueue;
use aiome_core::error::AiomeError;
use aiome_core::expression::Expression;
use aiome_core::traits::JobQueue;
use async_trait::async_trait;
use sqlx::Row;

#[async_trait]
pub trait ExpressionOps {
    async fn store_expression(&self, expression: &Expression) -> Result<(), AiomeError>;
    async fn fetch_expressions(&self, limit: i64) -> Result<Vec<Expression>, AiomeError>;
    async fn get_auto_expression_enabled(&self) -> Result<bool, AiomeError>;
    async fn set_auto_expression_enabled(&self, enabled: bool) -> Result<(), AiomeError>;
}

#[async_trait]
impl ExpressionOps for SqliteJobQueue {
    async fn store_expression(&self, expression: &Expression) -> Result<(), AiomeError> {
        let karma_refs_json =
            serde_json::to_string(&expression.karma_refs).unwrap_or_else(|_| "[]".to_string());

        sqlx::query(
            "INSERT INTO expressions (id, content, emotion, karma_refs, created_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&expression.id)
        .bind(&expression.content)
        .bind(&expression.emotion)
        .bind(&karma_refs_json)
        .bind(&expression.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to store expression: {}", e) })?;

        Ok(())
    }

    async fn fetch_expressions(&self, limit: i64) -> Result<Vec<Expression>, AiomeError> {
        let rows = sqlx::query(
            "SELECT id, content, emotion, karma_refs, created_at FROM expressions ORDER BY created_at DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to fetch expressions: {}", e) })?;

        let mut results = Vec::new();
        for row in rows {
            let karma_refs_str: String = row.get("karma_refs");
            let karma_refs: Vec<String> = serde_json::from_str(&karma_refs_str).unwrap_or_default();

            results.push(Expression {
                id: row.get("id"),
                content: row.get("content"),
                emotion: row.get("emotion"),
                karma_refs,
                created_at: row.get("created_at"),
            });
        }

        Ok(results)
    }

    async fn get_auto_expression_enabled(&self) -> Result<bool, AiomeError> {
        let row =
            sqlx::query("SELECT value FROM system_settings WHERE key = 'auto_expression_enabled'")
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to fetch setting: {}", e),
                })?;

        if let Some(r) = row {
            let val: String = r.get("value");
            Ok(val == "true")
        } else {
            Ok(false)
        }
    }

    async fn set_auto_expression_enabled(&self, enabled: bool) -> Result<(), AiomeError> {
        sqlx::query(
            "INSERT OR REPLACE INTO system_settings (key, value, category, is_secret) VALUES ('auto_expression_enabled', ?, 'expression', 0)"
        )
        .bind(if enabled { "true" } else { "false" })
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to set setting: {}", e) })?;

        Ok(())
    }
}
