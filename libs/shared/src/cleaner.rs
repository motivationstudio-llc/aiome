/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

//! # Cleaner — ストレージ清掃と監視
//!
//! タスク実行過程で発生する一時ファイルやキャッシュを自動清掃する。
//! また、ディスク残量を監視し、パンク前に安全に停止（安全弁）する機能を提供する。

use std::path::{Path, PathBuf};
use sysinfo::Disks;

/// クリーニング対象のディレクトリ情報
#[derive(Debug, Clone)]
pub struct CleanupTarget {
    pub path: PathBuf,
    pub recursive: bool,
}

/// ストレージ監視と清掃を行うクリーナー
pub struct StorageCleaner {
    targets: Vec<CleanupTarget>,
    threshold_percent: f32,
}

impl StorageCleaner {
    /// 新規クリーナー作成
    ///
    /// # Arguments
    /// * `targets` - 清掃対象のディレクトリリスト
    /// * `threshold_percent` - ディスク使用率の閾値（例: 90.0）
    pub fn new(targets: Vec<CleanupTarget>, threshold_percent: f32) -> Self {
        Self {
            targets,
            threshold_percent,
        }
    }

    /// ディスク使用率が閾値を超えているかチェックする
    ///
    /// # Returns
    /// 閾値を超えている（危険な状態）場合は `true`
    pub fn is_disk_full(&self) -> bool {
        let disks = Disks::new_with_refreshed_list();

        for disk in &disks {
            // ルートディレクトリを含むディスクをチェック（macOS の標準的な構成を想定）
            let mount_point = disk.mount_point();
            if mount_point == Path::new("/") || mount_point.starts_with("/System/Volumes/Data") {
                let used = disk.total_space() - disk.available_space();
                let usage_percent = (used as f32 / disk.total_space() as f32) * 100.0;
                
                if usage_percent > self.threshold_percent {
                    tracing::warn!(
                        "⚠️ Disk usage high: {:.2}% on {} (Threshold: {:.2}%)",
                        usage_percent,
                        mount_point.display(),
                        self.threshold_percent
                    );
                    return true;
                }
            }
        }
        false
    }

    /// 指定されたターゲットディレクトリ内のファイルを削除する
    pub fn cleanup(&self) -> Result<(), std::io::Error> {
        for target in &self.targets {
            if !target.path.exists() {
                continue;
            }

            tracing::info!("🧹 Cleaning up directory: {}", target.path.display());
            
            if target.recursive {
                for entry in std::fs::read_dir(&target.path)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() {
                        std::fs::remove_file(path)?;
                    } else if path.is_dir() {
                        std::fs::remove_dir_all(path)?;
                    }
                }
            } else {
                for entry in std::fs::read_dir(&target.path)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() {
                        std::fs::remove_file(path)?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_cleanup_files() {
        let temp_dir = std::env::temp_dir().join("aiome_core_test_cleanup");
        fs::create_dir_all(&temp_dir).unwrap();
        
        let file_path = temp_dir.join("temp_file.txt");
        fs::write(&file_path, "trash").unwrap();
        assert!(file_path.exists());

        let target = CleanupTarget {
            path: temp_dir.clone(),
            recursive: false,
        };
        let cleaner = StorageCleaner::new(vec![target], 90.0);
        cleaner.cleanup().unwrap();

        assert!(!file_path.exists());
        fs::remove_dir(temp_dir).unwrap();
    }
}
