/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

use super::SqliteJobQueue;
use aiome_core::error::AiomeError;
use async_trait::async_trait;
use base64::{prelude::BASE64_STANDARD, Engine};
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use sqlx::Row;
use tracing::warn;

#[async_trait]
pub trait SwarmOps {
    async fn do_get_node_id(&self) -> Result<String, AiomeError>;
    async fn do_sign_swarm_payload(&self, payload: &str) -> Result<String, AiomeError>;
    async fn do_tick_local_clock(&self) -> Result<u64, AiomeError>;
    async fn do_sync_local_clock(&self, remote_clock: u64) -> Result<u64, AiomeError>;
    async fn do_get_global_api_failures(&self) -> Result<i64, AiomeError>;
    async fn do_record_global_api_failure(&self) -> Result<i64, AiomeError>;
    async fn do_record_global_api_success(&self) -> Result<(), AiomeError>;
}

#[async_trait]
impl SwarmOps for SqliteJobQueue {
    async fn do_get_node_id(&self) -> Result<String, AiomeError> {
        let row = sqlx::query("SELECT value FROM system_state WHERE key = 'node_id'")
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

        if let Some(r) = row {
            Ok(r.get("value"))
        } else {
            let mut csprng = OsRng;
            let signing_key = SigningKey::generate(&mut csprng);
            let pubkey_b64 = BASE64_STANDARD.encode(signing_key.verifying_key().as_bytes());
            let privkey_b64 = BASE64_STANDARD.encode(signing_key.to_bytes());

            let mut tx = self
                .pool
                .begin()
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: e.to_string(),
                })?;
            sqlx::query("INSERT INTO system_state (key, value) VALUES ('node_id', ?)")
                .bind(&pubkey_b64)
                .execute(&mut *tx)
                .await
                .ok();
            sqlx::query("INSERT INTO system_state (key, value) VALUES ('node_privkey', ?)")
                .bind(&privkey_b64)
                .execute(&mut *tx)
                .await
                .ok();
            tx.commit().await.ok();

            Ok(pubkey_b64)
        }
    }

    async fn do_sign_swarm_payload(&self, payload: &str) -> Result<String, AiomeError> {
        let row = sqlx::query("SELECT value FROM system_state WHERE key = 'node_privkey'")
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

        if let Some(r) = row {
            let privkey_b64: String = r.get("value");
            let priv_bytes =
                BASE64_STANDARD
                    .decode(privkey_b64)
                    .map_err(|_| AiomeError::Infrastructure {
                        reason: "Corrupt node key".to_string(),
                    })?;
            let mut key_arr = [0u8; 32];
            key_arr.copy_from_slice(&priv_bytes);
            let signing_key = SigningKey::from_bytes(&key_arr);
            let signature = signing_key.sign(payload.as_bytes());
            Ok(BASE64_STANDARD.encode(signature.to_bytes()))
        } else {
            let _ = self.do_get_node_id().await?; // Ensure key exists
            Box::pin(self.do_sign_swarm_payload(payload)).await
        }
    }

    async fn do_tick_local_clock(&self) -> Result<u64, AiomeError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;
        let current: i64 =
            sqlx::query("SELECT value FROM system_state WHERE key = 'logical_clock'")
                .fetch_one(&mut *tx)
                .await
                .map(|r| r.get::<String, _>("value").parse().unwrap_or(0))
                .unwrap_or(0);

        let next = current + 1;
        sqlx::query("UPDATE system_state SET value = ? WHERE key = 'logical_clock'")
            .bind(next.to_string())
            .execute(&mut *tx)
            .await
            .ok();
        tx.commit().await.ok();
        Ok(next as u64)
    }

    async fn do_sync_local_clock(&self, remote_clock: u64) -> Result<u64, AiomeError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;
        let current: i64 =
            sqlx::query("SELECT value FROM system_state WHERE key = 'logical_clock'")
                .fetch_one(&mut *tx)
                .await
                .map(|r| r.get::<String, _>("value").parse().unwrap_or(0))
                .unwrap_or(0);

        // Gap 3 Mitigation: Clock Skew Rejection
        if remote_clock > (current as u64) + 100_000 {
            warn!(
                "⚠️ Potential Clock Poisoning attempt or severe skew detected: {} vs {}",
                remote_clock, current
            );
            tx.rollback().await.ok();
            return Ok(current as u64);
        }

        let next = std::cmp::max(current as u64, remote_clock) + 1;
        sqlx::query("UPDATE system_state SET value = ? WHERE key = 'logical_clock'")
            .bind(next.to_string())
            .execute(&mut *tx)
            .await
            .ok();
        tx.commit().await.ok();
        Ok(next)
    }

    async fn do_get_global_api_failures(&self) -> Result<i64, AiomeError> {
        let row =
            sqlx::query("SELECT value FROM system_state WHERE key = 'consecutive_api_failures'")
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to read system_state: {}", e),
                })?;

        if let Some(r) = row {
            let val_str: String = r.try_get("value").unwrap_or_default();
            Ok(val_str.parse().unwrap_or(0))
        } else {
            Ok(0)
        }
    }

    async fn do_record_global_api_failure(&self) -> Result<i64, AiomeError> {
        let current = self.do_get_global_api_failures().await?;
        let next = current + 1;

        sqlx::query(
            "INSERT INTO system_state (key, value, updated_at) 
             VALUES ('consecutive_api_failures', ?, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at"
        )
        .bind(next.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to update system_state: {}", e) })?;

        Ok(next)
    }

    async fn do_record_global_api_success(&self) -> Result<(), AiomeError> {
        sqlx::query(
            "INSERT INTO system_state (key, value, updated_at) 
             VALUES ('consecutive_api_failures', '0', datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to reset system_state: {}", e) })?;

        Ok(())
    }
}
