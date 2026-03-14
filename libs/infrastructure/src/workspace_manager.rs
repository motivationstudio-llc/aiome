/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

//! # Workspace Manager — The Caretaker
//!
//! 物理ファイルシステムへの「納品」と「清掃」を担う独立モジュール。
//! - Delivery (Safe Move Protocol v2): アトミックリネーム、0バイト防御、UUIDプレフィックス付与。
//! - Scavenger (Deep Cleansing v2): 再帰探査、拡張子ホワイトリスト、ゴーストタウン（空フォルダ）の枝打ち。
//!
//! [The Absolute Silence Audit 通過済設計]

use aiome_core::error::AiomeError;
use async_recursion::async_recursion;
use chrono::Utc;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs;
use tracing::{error, info, warn};

pub struct WorkspaceManager;

impl WorkspaceManager {
    /// Safe Move Protocol v2: 完成品を安全に納品先に移動させる
    ///
    /// 1. サイズ検証 (0バイト拒否)
    /// 2. バッファフラッシュ待ち (2s sleep)
    /// 3. 衝突回避 (UUID+Timestamp プレフィックス)
    /// 4. アトミック移動 (rename / fallback copy+remove)
    pub async fn deliver_output(
        job_id: &str,
        source_path: &Path,
        export_dir: &str,
    ) -> Result<PathBuf, AiomeError> {
        let export_path = PathBuf::from(export_dir);

        // 納品先ディレクトリの確保
        if !export_path.exists() {
            fs::create_dir_all(&export_path)
                .await
                .map_err(|e| AiomeError::Infrastructure {
                    reason: format!("Failed to create export dir: {}", e),
                })?;
        }

        // 1. サイズ検証 (Hollow Artifact 防止)
        let metadata = fs::metadata(source_path)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Source file missing or inaccessible: {}", e),
            })?;

        if metadata.len() == 0 {
            return Err(AiomeError::Infrastructure {
                reason:
                    "Safe Move Protocol: Source file size is 0 bytes (Hollow Artifact blocked)."
                        .into(),
            });
        }

        // 2. バッファフラッシュ待ち
        // （別プロセスの非同期I/OやOSのAPFS遅延書き込み完了を物理的に待機）
        tokio::time::sleep(Duration::from_secs(2)).await;

        // 再度メタデータを確認し、書き込みが継続していないかチェック（オプショナルだが安全）
        let metadata_after = fs::metadata(source_path).await.unwrap_or(metadata);
        if metadata_after.len() == 0 {
            return Err(AiomeError::Infrastructure {
                reason: "Safe Move Protocol: File became 0 bytes after wait.".into(),
            });
        }

        // 3. 衝突回避 (Unique Artifact Naming)
        let now_str = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let original_name = source_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("artifact");

        let unique_filename = format!("{}_{}_{}", now_str, job_id, original_name);
        let dest_path = export_path.join(&unique_filename);

        info!(
            "🚚 The Delivery: Executing Safe Move -> {}",
            dest_path.display()
        );

        // 4. アトミック移動 (Rename with Fallback)
        match fs::rename(source_path, &dest_path).await {
            Ok(_) => {
                info!("✅ Safe Move (Atomic Rename) Success.");
                Ok(dest_path)
            }
            Err(e) => {
                warn!("⚠️ Atomic Rename failed (likely cross-device EXDEV). Fallback to copy+remove: {}", e);
                // フォールバック: コピーして削除
                fs::copy(source_path, &dest_path).await.map_err(|ce| {
                    AiomeError::Infrastructure {
                        reason: format!("Safe Move Fallback Copy Failed: {}", ce),
                    }
                })?;

                // コピー後のサイズ等検証も可能だが、ここでは単純に元を消す
                fs::remove_file(source_path).await.map_err(|re| {
                    error!("❌ Safe Move: Copied successfully, but failed to remove source. Orphan left behind: {}", re);
                    AiomeError::Infrastructure {
                        reason: format!("Failed to clean up source after copy: {}", re),
                    }
                })?;

                info!("✅ Safe Move (Fallback Copy) Success.");
                Ok(dest_path)
            }
        }
    }

    /// Deep Cleansing v2 (The Scavenger)
    ///
    /// 再帰的に探索し、古い対象ファイルを削除。帰りがけに空ディレクトリを枝打ち（Pruning）する。
    pub async fn cleanup_expired_files(
        dir: &str,
        clean_after_hours: u64,
        allowed_extensions: &[&str],
    ) -> Result<(), AiomeError> {
        let root = PathBuf::from(dir);
        if !root.exists() {
            return Ok(());
        }

        info!(
            "🧹 The Scavenger: Commencing Deep Cleansing in {}",
            root.display()
        );
        let (files_deleted, dirs_pruned) =
            Self::recursive_clean(&root, clean_after_hours, allowed_extensions, true).await?;
        info!(
            "🧹 The Scavenger: Cleansing complete. {} files deleted, {} directories pruned.",
            files_deleted, dirs_pruned
        );

        Ok(())
    }

    /// Returns (files_deleted_count, dirs_pruned_count)
    #[async_recursion]
    async fn recursive_clean(
        dir: &Path,
        clean_after_hours: u64,
        allowed_extensions: &[&str],
        is_root: bool,
    ) -> Result<(u64, u64), AiomeError> {
        let mut read_dir = fs::read_dir(dir)
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to read dir {}: {}", dir.display(), e),
            })?;

        let mut files_deleted = 0;
        let mut dirs_pruned = 0;
        let mut has_contents = false; // To determine if directory is empty *after* processing

        while let Some(entry) = read_dir.next_entry().await.unwrap_or(None) {
            let path = entry.path();
            let metadata = match fs::metadata(&path).await {
                Ok(m) => m,
                Err(_) => {
                    has_contents = true; // Error reading, better to assume it's kept
                    continue;
                }
            };

            if metadata.is_dir() {
                // Recursive step downward (Depth-First Search)
                let (f_del, d_prune) = Box::pin(Self::recursive_clean(
                    &path,
                    clean_after_hours,
                    allowed_extensions,
                    false,
                ))
                .await?;
                files_deleted += f_del;
                dirs_pruned += d_prune;

                // If the child directory wasn't pruned, then this directory still has contents
                if path.exists() {
                    has_contents = true;
                }
            } else if metadata.is_file() {
                // Validate file for deletion
                let is_expired = match metadata.modified() {
                    Ok(mod_time) => {
                        if let Ok(elapsed) = mod_time.elapsed() {
                            elapsed.as_secs() > clean_after_hours * 3600
                        } else {
                            false // Time drift, safe side
                        }
                    }
                    Err(_) => false, // Cannot read time, safe side
                };

                let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

                // Extension whitelist logic:
                // Either match purely the extension string ("dat") or with dot (".dat")
                let ext_normalized = format!(".{}", extension);
                let ext_matched = allowed_extensions
                    .iter()
                    .any(|&ae| ae == ext_normalized || ae == extension);

                if is_expired && ext_matched {
                    match fs::remove_file(&path).await {
                        Ok(_) => {
                            files_deleted += 1;
                        }
                        Err(e) => {
                            error!(
                                "❌ The Scavenger: Failed to delete expired file {}: {}",
                                path.display(),
                                e
                            );
                            has_contents = true;
                        }
                    }
                } else {
                    // Protected by time or whitelist
                    has_contents = true;
                }
            } else {
                // Symlink or other types, leave alone
                has_contents = true;
            }
        }

        // Post-order Pruning (Ghost Town Prevention)
        // Never prune the root directory that was initially passed to cleanup_expired_files.
        if !has_contents && !is_root {
            match fs::remove_dir(dir).await {
                Ok(_) => {
                    dirs_pruned += 1;
                }
                Err(e) => {
                    // Could be recreating while we delete, just ignore
                    warn!(
                        "⚠️ The Scavenger: Could not prune directory {}: {}",
                        dir.display(),
                        e
                    );
                }
            }
        }

        Ok((files_deleted, dirs_pruned))
    }
}
