/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

use super::try_get_optional_string;
use super::SqliteJobQueue;
use aiome_core::contracts::{ArenaMatch, FederatedKarma, ImmuneRule};
use aiome_core::error::AiomeError;
use aiome_core::traits::JobQueue;
use async_trait::async_trait;
use sqlx::Row;

#[async_trait]
pub trait FederationOps {
    async fn do_export_federated_data(
        &self,
        since: Option<&str>,
    ) -> Result<(Vec<FederatedKarma>, Vec<ImmuneRule>, Vec<ArenaMatch>), AiomeError>;
    async fn do_import_federated_data(
        &self,
        karmas: Vec<FederatedKarma>,
        rules: Vec<ImmuneRule>,
        matches: Vec<ArenaMatch>,
    ) -> Result<(), AiomeError>;
    async fn do_get_peer_sync_time(&self, peer_url: &str) -> Result<Option<String>, AiomeError>;
    async fn do_update_peer_sync_time(
        &self,
        peer_url: &str,
        sync_time: &str,
    ) -> Result<(), AiomeError>;
    async fn do_fetch_unfederated_data(
        &self,
    ) -> Result<(Vec<FederatedKarma>, Vec<ImmuneRule>), AiomeError>;
    async fn do_mark_as_federated(
        &self,
        karma_ids: Vec<String>,
        rule_ids: Vec<String>,
    ) -> Result<(), AiomeError>;
}

#[async_trait]
impl FederationOps for SqliteJobQueue {
    async fn do_export_federated_data(
        &self,
        since: Option<&str>,
    ) -> Result<(Vec<FederatedKarma>, Vec<ImmuneRule>, Vec<ArenaMatch>), AiomeError> {
        let since_ts = since.unwrap_or("1970-01-01T00:00:00");

        let karmas = sqlx::query("SELECT id, job_id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, lamport_clock, node_id, signature FROM karma_logs WHERE created_at > ?")
            .bind(since_ts)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Export Karma failed: {}", e) })?;

        let rules = sqlx::query("SELECT id, pattern, severity, action, created_at, lamport_clock, node_id, signature FROM immune_rules WHERE created_at > ?")
            .bind(since_ts)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Export Rules failed: {}", e) })?;

        let matches = sqlx::query("SELECT id, skill_a, skill_b, topic, winner, reasoning, created_at FROM arena_history WHERE created_at > ?")
            .bind(since_ts)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Export Matches failed: {}", e) })?;

        let mut fed_karmas = Vec::new();
        for r in karmas {
            fed_karmas.push(FederatedKarma {
                id: r.get("id"),
                job_id: try_get_optional_string(&r, "job_id"),
                karma_type: r.get("karma_type"),
                related_skill: r.get("related_skill"),
                lesson: r.get("lesson"),
                weight: r.get::<i64, _>("weight") as i32,
                soul_version_hash: try_get_optional_string(&r, "soul_version_hash"),
                created_at: r.get("created_at"),
                lamport_clock: r.get::<i64, _>("lamport_clock") as u64,
                node_id: r.get("node_id"),
                signature: try_get_optional_string(&r, "signature"),
            });
        }

        let mut fed_rules = Vec::new();
        for r in rules {
            fed_rules.push(ImmuneRule {
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

        let mut fed_matches = Vec::new();
        for r in matches {
            fed_matches.push(ArenaMatch {
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

    async fn do_import_federated_data(
        &self,
        karmas: Vec<FederatedKarma>,
        rules: Vec<ImmuneRule>,
        matches: Vec<ArenaMatch>,
    ) -> Result<(), AiomeError> {
        if !karmas.is_empty() || !rules.is_empty() || !matches.is_empty() {
            info!(
                "📥 [Federation] Importing {} karmas, {} rules, {} matches.",
                karmas.len(),
                rules.len(),
                matches.len()
            );
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Import Tx start failed: {}", e),
            })?;

        use base64::{prelude::BASE64_STANDARD, Engine};
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};
        use tracing::{info, warn};

        for k in karmas {
            // Verify Ed25519 Signature
            let mut valid = false;
            if let Some(ref sig_b64) = k.signature {
                let payload = format!("{}:{}:{}", k.id, k.lesson, k.lamport_clock);
                if let (Ok(pubkey_bytes), Ok(sig_bytes)) = (
                    BASE64_STANDARD.decode(&k.node_id),
                    BASE64_STANDARD.decode(sig_b64),
                ) {
                    if let (Ok(pubkey), Ok(sig)) = (
                        VerifyingKey::from_bytes(&pubkey_bytes.try_into().unwrap_or([0; 32])),
                        Signature::from_slice(&sig_bytes),
                    ) {
                        if pubkey.verify(payload.as_bytes(), &sig).is_ok() {
                            valid = true;
                        } else {
                            warn!(
                                "🛡️ [Federation] Signature verification failed for Karma {}.",
                                k.id
                            );
                        }
                    }
                }
            }

            if !valid {
                warn!(
                    "🛡️ [Federation] Skipping Karma {} due to invalid/missing signature.",
                    k.id
                );
                continue;
            }

            let clean_lesson = if k.lesson.len() > 2000 {
                format!(
                    "{}... [Truncated for Swarm Safety]",
                    k.lesson.chars().take(2000).collect::<String>()
                )
            } else {
                k.lesson.clone()
            };

            let _ = self.sync_local_clock(k.lamport_clock).await;

            if let Err(e) = sqlx::query(
                "INSERT INTO karma_logs (id, job_id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, is_federated, lamport_clock, node_id, signature) 
                 VALUES (?, NULL, ?, ?, ?, ?, ?, ?, 1, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET 
                    lesson = excluded.lesson, 
                    weight = excluded.weight,
                    lamport_clock = excluded.lamport_clock,
                    node_id = excluded.node_id,
                    signature = excluded.signature,
                    is_federated = 1
                 WHERE excluded.lamport_clock > karma_logs.lamport_clock OR (excluded.lamport_clock = karma_logs.lamport_clock AND excluded.node_id > karma_logs.node_id)"
            )
            .bind(&k.id).bind(&k.karma_type).bind(&k.related_skill).bind(&clean_lesson)
            .bind(k.weight as i64).bind(&k.soul_version_hash).bind(&k.created_at)
            .bind(k.lamport_clock as i64).bind(&k.node_id).bind(&k.signature)
            .execute(&mut *tx).await {
                warn!("🛡️ [Federation] SQL Error importing karma {}: {:?}", k.id, e);
            }
        }

        for r in rules {
            // Verify Ed25519 Signature
            let mut valid = false;
            if let Some(ref sig_b64) = r.signature {
                let payload = format!("{}:{}:{}", r.id, r.pattern, r.lamport_clock);
                if let (Ok(pubkey_bytes), Ok(sig_bytes)) = (
                    BASE64_STANDARD.decode(&r.node_id),
                    BASE64_STANDARD.decode(sig_b64),
                ) {
                    if let (Ok(pubkey), Ok(sig)) = (
                        VerifyingKey::from_bytes(&pubkey_bytes.try_into().unwrap_or([0; 32])),
                        Signature::from_slice(&sig_bytes),
                    ) {
                        if pubkey.verify(payload.as_bytes(), &sig).is_ok() {
                            valid = true;
                        }
                    }
                }
            }

            if !valid {
                warn!(
                    "🛡️ [Federation] Skipping Rule {} due to invalid signature.",
                    r.id
                );
                continue;
            }

            let _ = self.sync_local_clock(r.lamport_clock).await;

            sqlx::query(
                "INSERT INTO immune_rules (id, pattern, severity, action, created_at, is_federated, lamport_clock, node_id, signature, status) 
                 VALUES (?, ?, ?, ?, ?, 1, ?, ?, ?, 'Quarantined')
                 ON CONFLICT(id) DO UPDATE SET 
                    pattern = excluded.pattern, 
                    severity = excluded.severity,
                    action = excluded.action,
                    lamport_clock = excluded.lamport_clock,
                    node_id = excluded.node_id,
                    signature = excluded.signature
                 WHERE excluded.lamport_clock > immune_rules.lamport_clock OR (excluded.lamport_clock = immune_rules.lamport_clock AND excluded.node_id > immune_rules.node_id)"
            )
            .bind(&r.id).bind(&r.pattern).bind(r.severity as i64).bind(&r.action).bind(&r.created_at)
            .bind(r.lamport_clock as i64).bind(&r.node_id).bind(&r.signature)
            .execute(&mut *tx).await.ok();
        }

        for m in matches {
            sqlx::query("INSERT INTO arena_history (id, skill_a, skill_b, topic, winner, reasoning, created_at) VALUES (?, ?, ?, ?, ?, ?, ?) ON CONFLICT(id) DO NOTHING")
                .bind(&m.id).bind(&m.skill_a).bind(&m.skill_b).bind(&m.topic).bind(&m.winner).bind(&m.reasoning).bind(&m.created_at)
                .execute(&mut *tx).await.ok();
        }

        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Import Tx commit failed: {}", e),
        })?;
        Ok(())
    }

    async fn do_get_peer_sync_time(&self, peer_url: &str) -> Result<Option<String>, AiomeError> {
        let row = sqlx::query("SELECT last_sync_at FROM federation_peers WHERE peer_url = ?")
            .bind(peer_url)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Get Peer sync time failed: {}", e),
            })?;

        Ok(row.map(|r| r.get("last_sync_at")))
    }

    async fn do_update_peer_sync_time(
        &self,
        peer_url: &str,
        sync_time: &str,
    ) -> Result<(), AiomeError> {
        sqlx::query("INSERT INTO federation_peers (peer_url, last_sync_at) VALUES (?, ?) ON CONFLICT(peer_url) DO UPDATE SET last_sync_at = excluded.last_sync_at")
            .bind(peer_url)
            .bind(sync_time)
            .execute(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Update Peer sync time failed: {}", e) })?;
        Ok(())
    }

    async fn do_fetch_unfederated_data(
        &self,
    ) -> Result<(Vec<FederatedKarma>, Vec<ImmuneRule>), AiomeError> {
        let karmas = sqlx::query("SELECT id, job_id, karma_type, related_skill, lesson, weight, soul_version_hash, created_at, lamport_clock, node_id, signature FROM karma_logs WHERE is_federated = 0")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Fetch unfederated karma failed: {}", e) })?;

        let rules = sqlx::query("SELECT id, pattern, severity, action, created_at, lamport_clock, node_id, signature FROM immune_rules WHERE is_federated = 0")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Fetch unfederated rules failed: {}", e) })?;

        let mut fed_karmas = Vec::new();
        for r in karmas {
            fed_karmas.push(FederatedKarma {
                id: r.get("id"),
                job_id: try_get_optional_string(&r, "job_id"),
                karma_type: r.get("karma_type"),
                related_skill: r.get("related_skill"),
                lesson: r.get("lesson"),
                weight: r.get::<i64, _>("weight") as i32,
                soul_version_hash: try_get_optional_string(&r, "soul_version_hash"),
                created_at: r.get("created_at"),
                lamport_clock: r.get::<i64, _>("lamport_clock") as u64,
                node_id: r.get("node_id"),
                signature: try_get_optional_string(&r, "signature"),
            });
        }

        let mut fed_rules = Vec::new();
        for r in rules {
            fed_rules.push(ImmuneRule {
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

        Ok((fed_karmas, fed_rules))
    }

    async fn do_mark_as_federated(
        &self,
        karma_ids: Vec<String>,
        rule_ids: Vec<String>,
    ) -> Result<(), AiomeError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Mark federated Tx failed: {}", e),
            })?;

        for id in karma_ids {
            sqlx::query("UPDATE karma_logs SET is_federated = 1 WHERE id = ?")
                .bind(id)
                .execute(&mut *tx)
                .await
                .ok();
        }
        for id in rule_ids {
            sqlx::query("UPDATE immune_rules SET is_federated = 1 WHERE id = ?")
                .bind(id)
                .execute(&mut *tx)
                .await
                .ok();
        }

        tx.commit().await.map_err(|e| AiomeError::Infrastructure {
            reason: format!("Mark federated commit failed: {}", e),
        })?;
        Ok(())
    }
}
