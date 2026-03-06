/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

//! # Workspace Manager Tests
//!
//! `workspace_manager.rs` の単体テスト。
//! - Ghost Town Check (再帰的枝打ち)
//! - Friendly Fire Check (拡張子ホワイトリスト)
//! - Safe Move Protocol

#[cfg(test)]
mod tests {
    use crate::workspace_manager::WorkspaceManager;
    use std::time::{SystemTime, Duration};
    use tokio::fs;

    #[tokio::test]
    async fn test_ghost_town_pruning() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let root = tmp_dir.path();

        // ディレクトリ構造: root / a / b / target.dat
        let dir_a = root.join("a");
        let dir_b = dir_a.join("b");
        fs::create_dir_all(&dir_b).await.unwrap();

        let file_path = dir_b.join("target.dat");
        fs::write(&file_path, "dummy").await.unwrap();

        // Modify file time to be 48 hours old
        let forty_eight_hours_ago = SystemTime::now() - Duration::from_secs(48 * 3600);
        filetime::set_file_mtime(
            &file_path,
            filetime::FileTime::from_system_time(forty_eight_hours_ago),
        ).unwrap();

        let allowed = [".dat"];
        WorkspaceManager::cleanup_expired_files(root.to_str().unwrap(), 24, &allowed).await.unwrap();

        // 期待: target.dat が消え、a/b が空になり消沈、a も空になり消沈
        assert!(!file_path.exists(), "target.dat should be deleted");
        assert!(!dir_b.exists(), "dir_b should be pruned");
        assert!(!dir_a.exists(), "dir_a should be pruned");
        assert!(root.exists(), "root should survive"); // root は prune しない
    }

    #[tokio::test]
    async fn test_friendly_fire_protection() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let root = tmp_dir.path();

        let target_ext = root.join("old.dat");
        let safe_ext = root.join("important.txt");
        let safe_time = root.join("new.dat");

        fs::write(&target_ext, "dummy").await.unwrap();
        fs::write(&safe_ext, "don't delete me").await.unwrap();
        fs::write(&safe_time, "just created").await.unwrap();

        let forty_eight_hours_ago = SystemTime::now() - Duration::from_secs(48 * 3600);
        
        filetime::set_file_mtime(&target_ext, filetime::FileTime::from_system_time(forty_eight_hours_ago)).unwrap();
        filetime::set_file_mtime(&safe_ext, filetime::FileTime::from_system_time(forty_eight_hours_ago)).unwrap();

        let allowed = [".dat"];
        WorkspaceManager::cleanup_expired_files(root.to_str().unwrap(), 24, &allowed).await.unwrap();

        assert!(!target_ext.exists(), "old.dat should be deleted");
        assert!(safe_ext.exists(), "important.txt should NOT be deleted (not in whitelist)");
        assert!(safe_time.exists(), "new.dat should NOT be deleted (not expired)");
    }

    #[tokio::test]
    async fn test_safe_move_protocol() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let source_dir = tmp_dir.path().join("source");
        let export_dir = tmp_dir.path().join("export");
        
        fs::create_dir_all(&source_dir).await.unwrap();
        
        // 0 byte file - should fail
        let empty_file = source_dir.join("empty.dat");
        fs::write(&empty_file, "").await.unwrap();
        let result = WorkspaceManager::deliver_output("job1", &empty_file, export_dir.to_str().unwrap()).await;
        assert!(result.is_err(), "Should reject 0 byte files");

        // Valid file
        let valid_file = source_dir.join("valid.dat");
        fs::write(&valid_file, "data").await.unwrap();
        
        let dest_path = WorkspaceManager::deliver_output("job2", &valid_file, export_dir.to_str().unwrap()).await.unwrap();
        
        assert!(!valid_file.exists(), "Source should be removed");
        assert!(dest_path.exists(), "Destination should exist");
        assert!(dest_path.file_name().unwrap().to_str().unwrap().contains("_job2_valid.dat"));
    }
}
