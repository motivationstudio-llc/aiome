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
    async fn do_sync_samsara_level(&self) -> Result<Option<aiome_core::contracts::SamsaraEvent>, AiomeError>;
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
        
        // Auto-sync level after adding exp
        let _ = self.do_sync_samsara_level().await;
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

    async fn do_sync_samsara_level(&self) -> Result<Option<aiome_core::contracts::SamsaraEvent>, AiomeError> {
        // 1. Calculate total Technical Karma weight (undeprecated)
        // Note: Technical Karma is the "Real World" anchor for growth.
        let total_weight_row = sqlx::query("SELECT SUM(weight) as total FROM karma_logs WHERE karma_type = 'Technical'")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to sum karma weight: {}", e) })?;
        
        let total_weight: i64 = total_weight_row.get::<Option<i64>, _>("total").unwrap_or(0);
        
        // 2. Get current stats
        let stats = self.do_get_agent_stats().await?;
        let mut current_level = stats.level;
        let original_level = current_level;

        // 3. Level-up Logic: Quadratic Threshold
        // Level 1 -> 2: 1000 weight
        // Level 2 -> 3: 4000 weight (total)
        // Level N: N^2 * 1000
        while total_weight >= (current_level as i64 * current_level as i64 * 1000) {
            current_level += 1;
            if current_level >= 100 { break; } // Safety cap
        }

        if current_level > original_level {
            sqlx::query("UPDATE agent_stats SET level = ?, updated_at = datetime('now') WHERE id = 1")
                .bind(current_level)
                .execute(&self.pool)
                .await
                .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to update level: {}", e) })?;
            
            tracing::info!("🌟 [SamsaraEngine] Level Up! {} -> {}", original_level, current_level);
            return Ok(Some(aiome_core::contracts::SamsaraEvent::LevelUp {
                old_level: original_level,
                new_level: current_level,
            }));
        }

        Ok(None)
    }
}
