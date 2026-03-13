/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 */

use async_trait::async_trait;
use aiome_core::error::AiomeError;
use aiome_core::traits::{ArtifactStore, ArtifactMeta, ArtifactCategory, ArtifactFile, CreateArtifactRequest};
use shared::sandbox::PathSandbox;
use sqlx::{SqlitePool, Row};
use uuid::Uuid;
use sha2::{Sha256, Digest};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use aiome_core::llm_provider::EmbeddingProvider;
use tracing::{info, warn};

pub struct SqliteArtifactStore {
    pool: SqlitePool,
    base_dir: PathBuf, // workspace/artifacts
    embed_provider: Option<Arc<dyn EmbeddingProvider>>,
}

impl SqliteArtifactStore {
    pub fn new(pool: SqlitePool, base_dir: PathBuf) -> Self {
        Self { pool, base_dir, embed_provider: None }
    }

    pub fn with_embeddings(mut self, provider: Arc<dyn EmbeddingProvider>) -> Self {
        self.embed_provider = Some(provider);
        self
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
        
        // 1. Create directory using PathSandbox for secure jail confinement
        let sandbox = PathSandbox::new(jail.root())
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to initialize sandbox: {}", e) })?;
            
        let relative_dir = Path::new("artifacts").join(&dir_name);
        let full_dir = sandbox.validate_path(&relative_dir)
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Security violation - invalid artifact path: {}", e) })?;
        
        std::fs::create_dir_all(&full_dir)
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to create artifact dir: {}", e) })?;

        let mut artifact_files = Vec::new();
        let mut manifest_hasher = Sha256::new();

        // 2. Save files and calculate hashes
        for (filename, content, mime_type) in req.files {
            let hash = Self::calculate_hash(&content);
            let file_path = sandbox.validate_path(full_dir.join(&filename))
                .map_err(|e| AiomeError::Infrastructure { reason: format!("Security violation - invalid file path: {}", e) })?;
            
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

        let file_manifest_json = serde_json::to_string(&artifact_files).expect("Failed to serialize artifact files");
        let tags_json = serde_json::to_string(&req.tags).expect("Failed to serialize tags");
        let karma_refs_json = serde_json::to_string(&req.karma_refs).expect("Failed to serialize karma refs");
        
        // SEC-6: Enforce payload size limits (max 500KB total for metadata)
        if file_manifest_json.len() + tags_json.len() + karma_refs_json.len() > 500 * 1024 {
            return Err(AiomeError::SecurityViolation { reason: "Artifact metadata exceeds safety limits (500KB)".into() });
        }

        // Phase 2: Generate Embedding
        let mut embedding_blob: Option<Vec<u8>> = None;
        if let Some(ref provider) = self.embed_provider {
            // Include text_content in embedding context for higher semantic quality
            let context = format!("{} {:?} {} {}", 
                req.title, 
                req.category, 
                req.tags.join(" "),
                req.text_content.as_deref().unwrap_or("")
            );
            match provider.embed(&context, false).await {
                Ok(vec) => {
                    info!("🧠 Generated embedding for artifact: {} ({} dims)", req.title, vec.len());
                    embedding_blob = Some(vec.iter().flat_map(|f| f.to_le_bytes()).collect());
                }
                Err(e) => warn!("⚠️ Failed to generate embedding for {}: {:?}", req.title, e),
            }
        }

        let signature = format!("{:x}", manifest_hasher.finalize());

        // 3. Store in SQLite
        sqlx::query(
            "INSERT INTO ai_artifacts (id, title, category, tags, created_by, dir_path, file_manifest, karma_refs, job_ref, signature, embedding, text_content) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(&req.title)
        .bind(serde_json::to_string(&req.category).expect("Failed to serialize category").replace("\"", ""))
        .bind(&tags_json)
        .bind(&req.created_by)
        .bind(relative_dir.to_str().unwrap_or_default())
        .bind(&file_manifest_json)
        .bind(&karma_refs_json)
        .bind(req.job_ref)
        .bind(&signature)
        .bind(embedding_blob)
        .bind(req.text_content)
        .execute(&self.pool)
        .await
        .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to store artifact metadata: {}", e) })?;

        // Phase 1: Store Edges (Provenance DAG)
        for edge_req in req.parent_refs {
            let edge_id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO artifact_edges (id, source_id, target_id, source_type, relation, metadata) VALUES (?, ?, ?, ?, ?, ?)"
            )
            .bind(&edge_id)
            .bind(&id)
            .bind(&edge_req.target_id)
            .bind(&edge_req.source_type)
            .bind(&edge_req.relation)
            .bind(serde_json::to_string(&edge_req.metadata.unwrap_or(serde_json::json!({}))).unwrap_or_default())
            .execute(&self.pool)
            .await
            .ok();
        }

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
            query = query.bind(serde_json::to_string(cat).expect("Failed to serialize category").replace("\"", ""));
        }

        let rows = query.fetch_all(&self.pool).await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to list artifacts: {}", e) })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(ArtifactMeta {
                id: row.get("id"),
                title: row.get("title"),
                category: serde_json::from_str(&format!("\"{}\"", row.get::<String, _>("category"))).unwrap_or(ArtifactCategory::Report),
                tags: serde_json::from_str(row.get("tags")).unwrap_or_default(),
                created_by: row.get("created_by"),
                dir_path: row.get("dir_path"),
                files: serde_json::from_str(row.get("file_manifest")).unwrap_or_default(),
                karma_refs: serde_json::from_str(row.get("karma_refs")).unwrap_or_default(),
                job_ref: row.get("job_ref"),
                soul_version_hash: row.get("soul_version_hash"),
                signature: row.get("signature"),
                text_content: row.get("text_content"),
                edges: Vec::new(), // Populated on-demand or with specific fetch
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

        if let Some(r) = row {
            let id: String = r.get("id");
            let edges = self.get_artifact_edges(&id).await.unwrap_or_default();

            Ok(Some(ArtifactMeta {
                id,
                title: r.get("title"),
                category: serde_json::from_str(&format!("\"{}\"", r.get::<String, _>("category"))).unwrap_or(ArtifactCategory::Report),
                tags: serde_json::from_str(r.get("tags")).unwrap_or_default(),
                created_by: r.get("created_by"),
                dir_path: r.get("dir_path"),
                files: serde_json::from_str(r.get("file_manifest")).unwrap_or_default(),
                karma_refs: serde_json::from_str(r.get("karma_refs")).unwrap_or_default(),
                job_ref: r.get("job_ref"),
                soul_version_hash: r.get("soul_version_hash"),
                signature: r.get("signature"),
                text_content: r.get("text_content"),
                edges,
                created_at: r.get("created_at"),
            }))
        } else {
            Ok(None)
        }
    }

    async fn read_artifact_file(&self, id: &str, filename: &str, jail: &bastion::fs_guard::Jail) -> Result<Vec<u8>, AiomeError> {
        let meta = self.fetch_artifact(id).await?
            .ok_or_else(|| AiomeError::ArtifactNotFound { path: id.to_string() })?;
        
        // Ensure the file is actually in the manifest
        if !meta.files.iter().any(|f| f.name == filename) {
            return Err(AiomeError::ArtifactNotFound { path: format!("{}/{}", id, filename) });
        }

        let sandbox = PathSandbox::new(jail.root())
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to initialize sandbox: {}", e) })?;
            
        let full_path = sandbox.validate_path(Path::new(&meta.dir_path).join(filename))
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Security violation - unauthorized file access: {}", e) })?;

        let content = std::fs::read(full_path)
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to read artifact file: {}", e) })?;
            
        Ok(content)
    }

    async fn delete_artifact(&self, id: &str, jail: &bastion::fs_guard::Jail) -> Result<(), AiomeError> {
        let meta = self.fetch_artifact(id).await?
            .ok_or_else(|| AiomeError::ArtifactNotFound { path: id.to_string() })?;

        // 1. Delete files from disk using sandbox
        let sandbox = PathSandbox::new(jail.root())
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to initialize sandbox: {}", e) })?;
            
        let full_dir = sandbox.validate_path(&meta.dir_path)
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Security violation - unauthorized directory access: {}", e) })?;

        for file in meta.files {
            let file_path = full_dir.join(&file.name);
            let _ = std::fs::remove_file(file_path);
        }
        
        // 2. Delete directory (if empty)
        let _ = std::fs::remove_dir(full_dir);

        // 3. Delete from DB
        sqlx::query("DELETE FROM ai_artifacts WHERE id = ?").bind(id).execute(&self.pool).await.ok();
        sqlx::query("DELETE FROM artifact_edges WHERE source_id = ? OR target_id = ?").bind(id).bind(id).execute(&self.pool).await.ok();

        Ok(())
    }

    async fn get_artifact_edges(&self, id: &str) -> Result<Vec<aiome_core::traits::ArtifactEdge>, AiomeError> {
        let rows = sqlx::query("SELECT * FROM artifact_edges WHERE source_id = ? OR target_id = ?")
            .bind(id).bind(id)
            .fetch_all(&self.pool).await
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        let mut edges = Vec::new();
        for r in rows {
            edges.push(aiome_core::traits::ArtifactEdge {
                id: r.get("id"),
                source_id: r.get("source_id"),
                target_id: r.get("target_id"),
                source_type: r.get("source_type"),
                relation: r.get("relation"),
                metadata: serde_json::from_str(r.get("metadata")).unwrap_or(serde_json::json!({})),
                created_at: r.get("created_at"),
            });
        }
        Ok(edges)
    }

    async fn add_artifact_edge(&self, edge: aiome_core::traits::ArtifactEdge) -> Result<(), AiomeError> {
        sqlx::query(
            "INSERT INTO artifact_edges (id, source_id, target_id, source_type, relation, metadata) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&edge.id).bind(&edge.source_id).bind(&edge.target_id).bind(&edge.source_type)
        .bind(&edge.relation).bind(serde_json::to_string(&edge.metadata).unwrap_or_default())
        .execute(&self.pool).await
        .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;
        Ok(())
    }

    async fn search_artifacts_semantic(&self, query: &str, category: Option<ArtifactCategory>, limit: i64) -> Result<Vec<ArtifactMeta>, AiomeError> {
        let provider = self.embed_provider.as_ref()
            .ok_or_else(|| AiomeError::Infrastructure { reason: "Embedding provider not configured for Semantic Search".into() })?;

        let query_vec = provider.embed(query, true).await?;
        let query_vec_f64: Vec<f64> = query_vec.iter().map(|&f| f as f64).collect();

        // 1. Build filtered query (SEC-7: Safety-clamp SQL fetch to avoid DoS)
        let mut sql = "SELECT id, title, category, tags, created_by, dir_path, file_manifest, embedding, text_content, created_at 
                       FROM ai_artifacts WHERE embedding IS NOT NULL".to_string();
        
        if category.is_some() {
            sql.push_str(" AND category = ?");
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT 1000");

        let mut db_query = sqlx::query(&sql);
        if let Some(ref cat) = category {
            let cat_str = serde_json::to_string(cat).expect("Failed to serialize category").replace("\"", "");
            db_query = db_query.bind(cat_str);
        }

        let rows = db_query.fetch_all(&self.pool).await
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        let mut candidates = Vec::new();
        for r in rows {
            let emb_bytes: Vec<u8> = r.get("embedding");
            let emb_vec: Vec<f64> = emb_bytes.chunks_exact(4)
                .map(|c| f32::from_le_bytes(c.try_into().expect("Invalid byte slice length")) as f64)
                .collect();
            
            let score = crate::job_queue::cosine_similarity(&query_vec_f64, &emb_vec);
            candidates.push((score, r));
        }

        candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        
        let mut results = Vec::new();
        for (_, r) in candidates.into_iter().take(limit as usize) {
            results.push(ArtifactMeta {
                id: r.get("id"),
                title: r.get("title"),
                category: serde_json::from_str(&format!("\"{}\"", r.get::<String, _>("category"))).unwrap_or(ArtifactCategory::Report),
                tags: serde_json::from_str(r.get("tags")).unwrap_or_default(),
                created_by: r.get("created_by"),
                dir_path: r.get("dir_path"),
                files: serde_json::from_str(r.get("file_manifest")).unwrap_or_default(),
                karma_refs: Vec::new(), 
                job_ref: None,
                soul_version_hash: None,
                signature: None,
                text_content: r.get("text_content"),
                edges: Vec::new(),
                created_at: r.get("created_at"),
            });
        }

        Ok(results)
    }
}
