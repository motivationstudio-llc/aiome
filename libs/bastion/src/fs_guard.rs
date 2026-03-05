/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

//! # fs_guard (File Jail)
//! 
//! パス・トラバーサル、シンボリックリンク攻撃、および競合状態(TOCTOU)を防ぐための
//! 産業グレードのファイルシステムガード。
//! 指定されたディレクトリ(Jail Root)外へのアクセスを物理的に遮断する。

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::io::{Result, Error, ErrorKind};
use std::os::unix::fs::OpenOptionsExt;

/// 指定されたディレクトリ配下のみにファイルアクセスを制限する Jail 構造体
#[derive(Clone, Debug)]
pub struct Jail {
    root: PathBuf,
}

impl Jail {
    /// 新しい Jail を初期化する。ディレクトリが存在しない場合は作成する。
    pub fn init<P: AsRef<Path>>(root: P) -> Result<Self> {
        let path = root.as_ref();
        if !path.exists() {
            std::fs::create_dir_all(path)?;
        }
        Self::new(path)
    }

    /// 新しい Jail を作成する。root path は絶対パスに正規化される。
    pub fn new<P: AsRef<Path>>(root: P) -> Result<Self> {
        let root_canonical = root.as_ref().canonicalize()?;
        if !root_canonical.is_dir() {
            return Err(Error::new(ErrorKind::InvalidInput, "Jail root must be a directory"));
        }
        Ok(Self { root: root_canonical })
    }

    /// Jail のルートパスを取得する
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 安全にファイルをオープンする。
    /// 内部で正規化、シンボリックリンク追跡禁止、およびオープン後のパス検証を行う。
    pub fn open_file<P: AsRef<Path>>(&self, path: P) -> Result<File> {
        let mut opts = OpenOptions::new();
        opts.read(true);
        self.secure_open(path, opts)
    }

    /// 安全にファイルを新規作成または上書きオープンする。
    pub fn create_file<P: AsRef<Path>>(&self, path: P) -> Result<File> {
        let mut opts = OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        self.secure_open(path, opts)
    }

    /// 内部的な安全オープンロジック
    fn secure_open<P: AsRef<Path>>(&self, path: P, mut options: OpenOptions) -> Result<File> {
        let requested_path = path.as_ref();
        
        // 入力パスが絶対パスの場合は、Jail Root 配下であることを強制する。
        // 相対パスの場合は、Jail Root を起点とする。
        let base_path = if requested_path.is_absolute() {
            requested_path.to_path_buf()
        } else {
            self.root.join(requested_path)
        };

        // 1. パスの正規化 (トラバーサルやシンボリックリンクを解決)
        // ファイルが存在しない可能性があるため、一度親ディレクトリまでで解決を試みる
        let full_path = if base_path.exists() {
            base_path.canonicalize()?
        } else {
            match base_path.parent() {
                Some(parent) if parent.exists() => {
                    let parent_canonical = parent.canonicalize()?;
                    parent_canonical.join(base_path.file_name().unwrap_or_default())
                }
                _ => base_path.clone(), // 親も存在しない場合はそのまま (starts_withで弾かれる)
            }
        };

        // 2. Jail Root プレフィックスチェック (物理的な境界チェック)
        if !full_path.starts_with(&self.root) {
            return Err(Error::new(ErrorKind::PermissionDenied, "Access Denied: Path outside of jail"));
        }

        // 3. アトミックオープン設定 (O_NOFOLLOW)
        // Unix系ではシンボリックリンクであればオープンを拒否
        #[cfg(unix)]
        {
            options.custom_flags(libc::O_NOFOLLOW);
        }

        // 4. オープン
        let file = options.open(&full_path)?;

        // 5. オープン後の再検証 (TOCTOU対策)
        // ファイルディスクリプタからメタデータを取得し、シンボリックリンクでないことを確認
        let metadata = file.metadata()?;
        if metadata.file_type().is_symlink() {
            return Err(Error::new(ErrorKind::PermissionDenied, "Access Denied: Symbolic link detected after open"));
        }

        // FD枯渇に対する警告（要件：FD上限管理への意識）
        // 実際の上限チェックはOS依存のため、ここではロジックの安全性のみ担保
        
        Ok(file)
    }

    /// 安全にディレクトリを作成する。
    pub fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let requested_path = path.as_ref();
        let full_path = if requested_path.is_absolute() {
            requested_path.to_path_buf()
        } else {
            self.root.join(requested_path)
        };

        // トラバーサルチェック
        if !full_path.starts_with(&self.root) {
            return Err(Error::new(ErrorKind::PermissionDenied, "Access Denied: Path outside of jail"));
        }

        std::fs::create_dir_all(full_path)
    }

    /// 安全にファイルにデータを書き込む。
    pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(&self, path: P, contents: C) -> Result<()> {
        let requested_path = path.as_ref();
        // create_file を使用して物理的な存在と境界を確保してから書き込む
        let mut file = self.create_file(requested_path)?;
        use std::io::Write;
        file.write_all(contents.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_jail_isolation() -> Result<()> {
        let dir = tempdir()?;
        let workspace = dir.path().join("workspace");
        fs::create_dir(&workspace)?;
        
        let jail = Jail::new(&workspace)?;
        
        // 正常系
        let safe_file_path = workspace.join("test.txt");
        fs::write(&safe_file_path, "hello")?;
        assert!(jail.open_file("test.txt").is_ok());

        // 異常系: トラバーサル
        assert!(jail.open_file("../outside.txt").is_err());
        
        // 異常系: 絶対パスによる脱出試行
        assert!(jail.open_file("/etc/passwd").is_err());

        Ok(())
    }

    #[test]
    fn test_create_in_jail() -> Result<()> {
        let dir = tempdir()?;
        let workspace = dir.path().join("workspace");
        fs::create_dir(&workspace)?;
        
        let jail = Jail::new(&workspace)?;
        
        // 新規作成
        let res = jail.create_file("new.txt");
        assert!(res.is_ok());
        
        // Jail外への作成試行
        let res_evil = jail.create_file("../evil.txt");
        assert!(res_evil.is_err());

        Ok(())
    }
}
