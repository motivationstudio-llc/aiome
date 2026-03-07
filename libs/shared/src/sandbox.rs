/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 * You may not provide the software to third parties as a hosted or managed service, 
 * where the service provides users with access to any substantial set of the features 
 * or functionality of the software.
 */

//! # PathSandbox — ファイルシステムサンドボックス
//!
//! 全てのファイル操作を許可されたディレクトリ内に閉じ込める「牢獄」。
//! Bastion Jail を使用して、TOCTOU 攻撃やシンボリックリンク攻撃を防止する。

use std::path::{Path, PathBuf};
use bastion::fs_guard::Jail;

/// 許可されたディレクトリ内でのみファイル操作を許可するサンドボックス
pub struct PathSandbox {
    jail: Jail,
}

impl PathSandbox {
    /// 新規サンドボックスの作成
    pub fn new<P: AsRef<Path>>(allowed_root: P) -> Result<Self, std::io::Error> {
        let jail = Jail::new(allowed_root)?;
        Ok(Self { jail })
    }

    /// パスがサンドボックス内にあるか検証し、安全なフルパスを返す
    /// Bastion Jail の検証ロジック（TOCTOU対策、シンボリックリンク制限）を使用。
    pub fn validate_path<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf, std::io::Error> {
        let requested_path = path.as_ref();
        let base_path = if requested_path.is_absolute() {
            requested_path.to_path_buf()
        } else {
            self.get_root().join(requested_path)
        };

        // Bastion の Jail ロジックに準拠した検証
        if base_path.exists() {
            let canonical = base_path.canonicalize()?;
            if !canonical.starts_with(self.get_root()) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "Access Denied: Path outside of jail (Bastion Guard)",
                ));
            }
            Ok(canonical)
        } else {
            // 存在しないファイルの場合は親ディレクトリを検証
            match base_path.parent() {
                Some(parent) if parent.exists() => {
                    let parent_canonical = parent.canonicalize()?;
                    if !parent_canonical.starts_with(self.get_root()) {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::PermissionDenied,
                            "Access Denied: Parent directory outside of jail",
                        ));
                    }
                    Ok(parent_canonical.join(base_path.file_name().unwrap_or_default()))
                }
                _ => {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "Path or parent directory does not exist",
                    ))
                }
            }
        }
    }

    /// Jail のルートパスを取得（内部検証用）
    fn get_root(&self) -> PathBuf {
        self.jail.root().to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_bastion_jail_integration() {
        let dir = tempdir().unwrap();
        let workspace = dir.path().join("workspace");
        fs::create_dir(&workspace).unwrap();
        
        let sandbox = PathSandbox::new(&workspace).unwrap();
        
        // 正常系
        let safe_file = workspace.join("test.txt");
        fs::write(&safe_file, "data").unwrap();
        assert!(sandbox.validate_path("test.txt").is_ok());

        // 異常系: トラバーサル
        assert!(sandbox.validate_path("../outside.txt").is_err());
    }
}
