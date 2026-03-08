/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 */

use async_trait::async_trait;
use aiome_core::error::AiomeError;
use aiome_core::contracts::{ImmuneRule, ArenaMatch};
use aiome_core::traits::JobQueue;
use sqlx::Row;
use super::SqliteJobQueue;
use super::try_get_optional_string;

#[async_trait]
pub trait GuardrailOps {
    async fn do_store_immune_rule(&self, rule: &ImmuneRule) -> Result<(), AiomeError>;
    async fn do_fetch_active_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError>;
    async fn do_get_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError>;
    async fn do_record_arena_match(&self, match_data: &ArenaMatch) -> Result<(), AiomeError>;
}

#[async_trait]
impl GuardrailOps for SqliteJobQueue {
    async fn do_store_immune_rule(&self, rule: &ImmuneRule) -> Result<(), AiomeError> {
        // --- Phase 10-B: Swarm Identity & Clock ---
        let node_id = self.get_node_id().await.unwrap_or_default();
        let clock = self.tick_local_clock().await.unwrap_or(0);
        let sign_target = format!("{}:{}:{}", rule.id, rule.pattern, clock);
        let signature = self.sign_swarm_payload(&sign_target).await.ok();

        sqlx::query("INSERT INTO immune_rules (id, pattern, severity, action, created_at, node_id, lamport_clock, signature) VALUES (?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(id) DO NOTHING")
            .bind(&rule.id)
            .bind(&rule.pattern)
            .bind(rule.severity as i64)
            .bind(&rule.action)
            .bind(&rule.created_at)
            .bind(&node_id)
            .bind(clock as i64)
            .bind(signature)
            .execute(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to store immune rule: {}", e) })?;
        Ok(())
    }

    async fn do_fetch_active_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError> {
        let rows = sqlx::query("SELECT id, pattern, severity, action, created_at, lamport_clock, node_id, signature FROM immune_rules WHERE status != 'Quarantined' ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to fetch immune rules: {}", e) })?;

        let mut rules = Vec::new();
        for r in rows {
            rules.push(ImmuneRule {
                id: r.get("id"),
                pattern: r.get("pattern"),
                severity: r.get::<i64, _>("severity") as u8,
                action: r.get("action"),
                created_at: r.get("created_at"),
                lamport_clock: r.get::<i64, _>("lamport_clock") as u64,
                node_id: r.get("node_id"),
                signature: try_get_optional_string(&r, "signature"),
            });
        }
        Ok(rules)
    }

    async fn do_get_immune_rules(&self) -> Result<Vec<ImmuneRule>, AiomeError> {
        let rows = sqlx::query("SELECT id, pattern, severity, action, created_at, lamport_clock, node_id, signature FROM immune_rules ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Fetch immune rules failed: {}", e) })?;

        let mut rules = Vec::new();
        for r in rows {
            rules.push(ImmuneRule {
                id: r.get("id"),
                pattern: r.get("pattern"),
                severity: r.get::<i64, _>("severity") as u8,
                action: r.get("action"),
                created_at: r.get("created_at"),
                lamport_clock: r.get::<i64, _>("lamport_clock") as u64,
                node_id: r.get("node_id"),
                signature: try_get_optional_string(&r, "signature"),
            });
        }
        Ok(rules)
    }

    async fn do_record_arena_match(&self, match_data: &ArenaMatch) -> Result<(), AiomeError> {
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
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to record arena match: {}", e) })?;
        Ok(())
    }
}
