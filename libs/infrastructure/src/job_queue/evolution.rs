/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 */

use async_trait::async_trait;
use aiome_core::error::AiomeError;
use sqlx::Row;
use super::SqliteJobQueue;

#[async_trait]
pub trait EvolutionOps {
    async fn do_get_agent_stats(&self) -> Result<shared::watchtower::AgentStats, AiomeError>;
    async fn do_add_resonance(&self, amount: i32) -> Result<(), AiomeError>;
    async fn do_add_tech_exp(&self, amount: i32) -> Result<(), AiomeError>;
    async fn do_add_creativity(&self, amount: i32) -> Result<(), AiomeError>;
    async fn do_record_soul_mutation(&self, old_hash: &str, new_hash: &str, reason: &str) -> Result<(), AiomeError>;
}

#[async_trait]
impl EvolutionOps for SqliteJobQueue {
    async fn do_get_agent_stats(&self) -> Result<shared::watchtower::AgentStats, AiomeError> {
        let row = sqlx::query("SELECT level, exp, resonance, creativity, fatigue FROM agent_stats WHERE id = 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to fetch agent stats: {}", e) })?;

        Ok(shared::watchtower::AgentStats {
            level: row.get("level"),
            exp: row.get("exp"),
            resonance: row.get("resonance"),
            creativity: row.get("creativity"),
            fatigue: row.get("fatigue"),
        })
    }

    async fn do_add_resonance(&self, amount: i32) -> Result<(), AiomeError> {
        sqlx::query("UPDATE agent_stats SET resonance = resonance + ?, updated_at = datetime('now') WHERE id = 1")
            .bind(amount)
            .execute(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to update resonance: {}", e) })?;
        Ok(())
    }

    async fn do_add_tech_exp(&self, amount: i32) -> Result<(), AiomeError> {
        sqlx::query("UPDATE agent_stats SET exp = exp + ?, updated_at = datetime('now') WHERE id = 1")
            .bind(amount)
            .execute(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to update exp: {}", e) })?;
        Ok(())
    }

    async fn do_add_creativity(&self, amount: i32) -> Result<(), AiomeError> {
        sqlx::query("UPDATE agent_stats SET creativity = creativity + ?, updated_at = datetime('now') WHERE id = 1")
            .bind(amount)
            .execute(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to update creativity: {}", e) })?;
        Ok(())
    }

    async fn do_record_soul_mutation(&self, old_hash: &str, new_hash: &str, reason: &str) -> Result<(), AiomeError> {
        sqlx::query("INSERT INTO soul_mutation_history (old_hash, new_hash, mutation_reason) VALUES (?, ?, ?)")
            .bind(old_hash)
            .bind(new_hash)
            .bind(reason)
            .execute(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
        Ok(())
    }
}
