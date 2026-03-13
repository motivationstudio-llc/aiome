/*
 * Aiome - The Autonomous AI Operating System
 */

use std::sync::Arc;
use std::path::{Path, PathBuf};
use sha2::{Sha256, Digest};
use aiome_core::error::AiomeError;
use aiome_core::traits::{ArtifactStore, CreateArtifactRequest, ArtifactCategory};
use sqlx::SqlitePool;
use tracing::{info, warn};
use bastion::fs_guard::Jail;

/// ProjectKnowledgeIndexer scans local documentation and indexes it for RAG.
/// NOTE: Secured by infrastructure layer. Standard std::fs is used here because 
/// it only targets internal project files (docs/, ARCHITECTURE.md) 
/// and not unknown user-uploaded content.
pub struct ProjectKnowledgeIndexer {
    artifact_store: Arc<dyn ArtifactStore>,
    pool: SqlitePool,
    workspace_root: PathBuf,
}

impl ProjectKnowledgeIndexer {
    pub fn new(artifact_store: Arc<dyn ArtifactStore>, pool: SqlitePool, workspace_root: PathBuf) -> Self {
        Self { artifact_store, pool, workspace_root }
    }

    pub async fn run_indexing(&self) -> Result<(), AiomeError> {
        info!("📚 [KnowledgeIndexer] Starting project knowledge indexing...");
        
        // Scan docs directory
        let docs_dir = self.workspace_root.join("docs");
        let arch_file = self.workspace_root.join("ARCHITECTURE.md");
        
        let mut files_to_index = Vec::new();
        if arch_file.exists() {
            files_to_index.push(arch_file);
        }
        
        if docs_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(docs_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().map(|e| e == "md").unwrap_or(false) {
                        files_to_index.push(path);
                    }
                }
            }
        }

        let jail = Jail::new("workspace").map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        for file_path in files_to_index {
            let relative_path = file_path.strip_prefix(&self.workspace_root)
                .unwrap_or(&file_path)
                .to_string_lossy()
                .to_string();
            
            if let Err(e) = self.index_file(&file_path, &relative_path, &jail).await {
                warn!("⚠️ [KnowledgeIndexer] Failed to index {}: {:?}", relative_path, e);
            }
        }

        info!("📚 [KnowledgeIndexer] Indexing cycle complete.");
        Ok(())
    }

    async fn index_file(&self, path: &Path, rel_path: &str, jail: &Jail) -> Result<(), AiomeError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to read {}: {}", rel_path, e) })?;
        
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        let state_key = format!("knowledge_hash_{}", rel_path);
        
        // Check if already indexed
        let existing_hash: Option<String> = sqlx::query_scalar("SELECT value FROM system_state WHERE key = ?")
            .bind(&state_key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        if existing_hash.as_deref() == Some(&hash) {
            return Ok(());
        }

        info!("📚 [KnowledgeIndexer] File changed: {}. Re-indexing...", rel_path);

        // 1. Delete old chunks
        let source_tag = format!("source:{}", rel_path);
        let old_ids: Vec<String> = sqlx::query_scalar("SELECT id FROM ai_artifacts WHERE tags LIKE ?")
            .bind(format!("%{}%", source_tag))
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        for id in old_ids {
            let _ = self.artifact_store.delete_artifact(&id, jail).await;
        }

        // 2. Chunk and Save
        let chunks = self.chunk_markdown(&content);
        for (i, (title, chunk_content)) in chunks.into_iter().enumerate() {
            let artifact_title = if title.is_empty() {
                format!("Knowledge: {} (Part {})", rel_path, i + 1)
            } else {
                format!("Knowledge: {} ({})", title, rel_path)
            };

            let req = CreateArtifactRequest {
                title: artifact_title,
                category: ArtifactCategory::Knowledge,
                tags: vec![source_tag.clone(), "rag".to_string()],
                created_by: "KnowledgeIndexer".to_string(),
                files: vec![(format!("chunk_{}.md", i), chunk_content.clone().into_bytes(), "text/markdown".to_string())],
                karma_refs: vec![],
                text_content: Some(chunk_content),
                job_ref: None,
                parent_refs: vec![],
            };

            self.artifact_store.save_artifact(req, jail).await?;
        }

        // 3. Update hash in system_state
        sqlx::query("INSERT INTO system_state (key, value, updated_at) VALUES (?, ?, datetime('now')) ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')")
            .bind(&state_key)
            .bind(&hash)
            .execute(&self.pool)
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        Ok(())
    }

    fn chunk_markdown(&self, content: &str) -> Vec<(String, String)> {
        let mut chunks = Vec::new();
        let mut current_title = String::new();
        let mut current_chunk = Vec::new();

        for line in content.lines() {
            if line.starts_with("## ") {
                if !current_chunk.is_empty() {
                    chunks.push((current_title.clone(), current_chunk.join("\n")));
                    current_chunk.clear();
                }
                current_title = line.trim_start_matches("## ").to_string();
            } else if line.starts_with("# ") && current_title.is_empty() {
                // Main title if no ## yet
                current_title = line.trim_start_matches("# ").to_string();
            }
            current_chunk.push(line);
        }

        if !current_chunk.is_empty() {
            chunks.push((current_title, current_chunk.join("\n")));
        }

        chunks
    }
}
