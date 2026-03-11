/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 */

use async_trait::async_trait;
use aiome_core::error::AiomeError;
use aiome_core::traits::{ArtifactStore, ArtifactMeta, ArtifactCategory, ArtifactFile, CreateArtifactRequest};
use sqlx::{SqlitePool, Row};
use uuid::Uuid;
use sha2::{Sha256, Digest};
use std::path::{Path, PathBuf};
use tracing::info;

pub struct SqliteArtifactStore {
    pool: SqlitePool,
    base_dir: PathBuf, // workspace/artifacts
}

impl SqliteArtifactStore {
    pub fn new(pool: SqlitePool, base_dir: PathBuf) -> Self {
        Self { pool, base_dir }
    }

    fn calculate_hash(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }
}

#[async_trait]
impl ArtifactStore for SqliteArtifactStore {
    async fn save_artifact(&self, req: CreateArtifactRequest, jail: &bastion::fs_guard::Jail) -> Result<String, AiomeError> {
        let id = Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now();
        let dir_name = format!("{}_{}", timestamp.format("%Y-%m-%d"), id[..8].to_string());
        
        // 1. Create directory using standard fs after validation
        let relative_dir = Path::new("artifacts").join(&dir_name);
        let full_dir = jail.root().join(&relative_dir);
        
        std::fs::create_dir_all(&full_dir)
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to create artifact dir: {}", e) })?;

        let mut artifact_files = Vec::new();
        let mut manifest_hasher = Sha256::new();

        // 2. Save files and calculate hashes
        for (filename, content, mime_type) in req.files {
            let hash = Self::calculate_hash(&content);
            let file_path = full_dir.join(&filename);
            
            std::fs::write(&file_path, &content)
                .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to write artifact file {}: {}", filename, e) })?;

            let file_meta = ArtifactFile {
                name: filename,
                mime_type,
                size_bytes: content.len() as u64,
                hash: hash.clone(),
            };
            
            manifest_hasher.update(hash.as_bytes());
            artifact_files.push(file_meta);
        }

        let file_manifest_json = serde_json::to_string(&artifact_files).unwrap();
        let tags_json = serde_json::to_string(&req.tags).unwrap();
        let karma_refs_json = serde_json::to_string(&req.karma_refs).unwrap();
        let signature = format!("{:x}", manifest_hasher.finalize());

        // 3. Store in SQLite
        sqlx::query(
            "INSERT INTO ai_artifacts (id, title, category, tags, created_by, dir_path, file_manifest, karma_refs, job_ref, signature) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(&req.title)
        .bind(serde_json::to_string(&req.category).unwrap().replace("\"", ""))
        .bind(&tags_json)
        .bind(&req.created_by)
        .bind(relative_dir.to_str().unwrap_or_default())
        .bind(&file_manifest_json)
        .bind(&karma_refs_json)
        .bind(req.job_ref)
        .bind(&signature)
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to store artifact metadata: {}", e) })?;

        info!("📦 Artifact saved: {} (ID: {})", req.title, id);
        Ok(id)
    }

    async fn list_artifacts(&self, category: Option<ArtifactCategory>, limit: i64) -> Result<Vec<ArtifactMeta>, AiomeError> {
        let sql = if let Some(ref cat) = category {
            format!("SELECT * FROM ai_artifacts WHERE category = ? ORDER BY created_at DESC LIMIT {}", limit)
        } else {
            format!("SELECT * FROM ai_artifacts ORDER BY created_at DESC LIMIT {}", limit)
        };

        let mut query = sqlx::query(&sql);
        if let Some(ref cat) = category {
            query = query.bind(serde_json::to_string(cat).unwrap().replace("\"", ""));
        }

        let rows = query.fetch_all(&self.pool).await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to list artifacts: {}", e) })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(ArtifactMeta {
                id: row.get("id"),
                title: row.get("id"),
                category: serde_json::from_str(&format!("\"{}\"", row.get::<String, _>("category"))).unwrap_or(ArtifactCategory::Report),
                tags: serde_json::from_str(row.get("tags")).unwrap_or_default(),
                created_by: row.get("created_by"),
                dir_path: row.get("dir_path"),
                files: serde_json::from_str(row.get("file_manifest")).unwrap_or_default(),
                karma_refs: serde_json::from_str(row.get("karma_refs")).unwrap_or_default(),
                job_ref: row.get("job_ref"),
                soul_version_hash: row.get("soul_version_hash"),
                signature: row.get("signature"),
                created_at: row.get("created_at"),
            });
        }

        Ok(results)
    }

    async fn fetch_artifact(&self, id: &str) -> Result<Option<ArtifactMeta>, AiomeError> {
        let row = sqlx::query("SELECT * FROM ai_artifacts WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to fetch artifact details: {}", e) })?;

        Ok(row.map(|r| ArtifactMeta {
            id: r.get("id"),
            title: r.get("id"),
            category: serde_json::from_str(&format!("\"{}\"", r.get::<String, _>("category"))).unwrap_or(ArtifactCategory::Report),
            tags: serde_json::from_str(r.get("tags")).unwrap_or_default(),
            created_by: r.get("created_by"),
            dir_path: r.get("dir_path"),
            files: serde_json::from_str(r.get("file_manifest")).unwrap_or_default(),
            karma_refs: serde_json::from_str(r.get("karma_refs")).unwrap_or_default(),
            job_ref: r.get("job_ref"),
            soul_version_hash: r.get("soul_version_hash"),
            signature: r.get("signature"),
            created_at: r.get("created_at"),
        }))
    }

    async fn read_artifact_file(&self, id: &str, filename: &str, jail: &bastion::fs_guard::Jail) -> Result<Vec<u8>, AiomeError> {
        let meta = self.fetch_artifact(id).await?
            .ok_or_else(|| AiomeError::ArtifactNotFound { path: id.to_string() })?;
        
        // Ensure the file is actually in the manifest
        if !meta.files.iter().any(|f| f.name == filename) {
            return Err(AiomeError::ArtifactNotFound { path: format!("{}/{}", id, filename) });
        }

        let full_path = jail.root().join(&meta.dir_path).join(filename);
        let content = std::fs::read(full_path)
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to read artifact file: {}", e) })?;
            
        Ok(content)
    }

    async fn delete_artifact(&self, id: &str, jail: &bastion::fs_guard::Jail) -> Result<(), AiomeError> {
        let meta = self.fetch_artifact(id).await?
            .ok_or_else(|| AiomeError::ArtifactNotFound { path: id.to_string() })?;

        // 1. Delete files from disk using standard fs after validation
        let full_dir = jail.root().join(&meta.dir_path);
        for file in meta.files {
            let file_path = full_dir.join(&file.name);
            let _ = std::fs::remove_file(file_path);
        }
        
        // 2. Delete directory (if empty)
        let _ = std::fs::remove_dir(full_dir);

        // 3. Delete from DB
        sqlx::query("DELETE FROM ai_artifacts WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to delete artifact metadata: {}", e) })?;

        Ok(())
    }
}
