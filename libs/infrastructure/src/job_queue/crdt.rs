/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use async_trait::async_trait;
use automerge::{AutoCommit, ReadDoc, transaction::Transactable};
use sqlx::Row;
use crate::job_queue::SqliteJobQueue;
use aiome_core::error::AiomeError;
use tracing::{info, error};

#[async_trait]
pub trait CrdtOps {
    async fn sync_timeline(&self, hub_id: &str, remote_blob: Option<&[u8]>) -> Result<Vec<u8>, AiomeError>;
    async fn get_timeline_blob(&self, hub_id: &str) -> Result<Option<Vec<u8>>, AiomeError>;
}

#[async_trait]
impl CrdtOps for SqliteJobQueue {
    /// [A-4] CRDT Timeline Sync
    /// Merges local timeline with remote timeline using Automerge.
    async fn sync_timeline(&self, hub_id: &str, remote_blob: Option<&[u8]>) -> Result<Vec<u8>, AiomeError> {
        let mut local_doc = match self.get_timeline_blob(hub_id).await? {
            Some(blob) => AutoCommit::load(&blob).map_err(|e| AiomeError::Infrastructure { reason: format!("Automerge load error: {}", e) })?,
            None => AutoCommit::new(),
        };

        if let Some(rb) = remote_blob {
            let mut remote_doc = AutoCommit::load(rb).map_err(|e| AiomeError::Infrastructure { reason: format!("Remote Automerge load error: {}", e) })?;
            local_doc.merge(&mut remote_doc).map_err(|e| AiomeError::Infrastructure { reason: format!("Automerge merge error: {}", e) })?;
        }

        // Add local marker if needed
        let now = chrono::Utc::now().to_rfc3339();
        local_doc.put(automerge::ROOT, "last_sync", now).ok();

        let finalized_blob = local_doc.save();
        
        sqlx::query("INSERT INTO timeline_checkpoints (id, automerge_blob, last_seq) VALUES (?, ?, ?) ON CONFLICT(id) DO UPDATE SET automerge_blob = ?, updated_at = datetime('now')")
            .bind(hub_id)
            .bind(&finalized_blob)
            .bind(0i64)
            .bind(&finalized_blob)
            .execute(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        Ok(finalized_blob)
    }

    async fn get_timeline_blob(&self, hub_id: &str) -> Result<Option<Vec<u8>>, AiomeError> {
        let row = sqlx::query("SELECT automerge_blob FROM timeline_checkpoints WHERE id = ?")
            .bind(hub_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
        
        Ok(row.map(|r| r.get(0)))
    }
}
