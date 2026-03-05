/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

//! # OS Utils — macOS 固有の安定化処理
//!
//! App Nap の防止、Spotlight インデックス対象外の設定など、
//! macOS 上での長時間稼働を安定させるためのユーティリティ。

use std::process::Command;
use std::path::Path;

/// macOS の省電力機能（App Nap）を無効化する
///
/// `caffeinate` コマンドを使用して、システムのアイドル状態や
/// プロセスの App Nap を防止する。戻り値の Child プロセスを保持する限り有効。
pub fn prevent_app_nap() -> Result<std::process::Child, std::io::Error> {
    #[cfg(target_os = "macos")]
    {
        tracing::info!("☕ Preventing App Nap and system sleep using 'caffeinate'...");
        // -i: prevent system idle sleep
        // -d: prevent display sleep (optional, but good for visibility)
        // -m: prevent disk idle sleep
        Command::new("caffeinate")
            .args(&["-i", "-m"])
            .spawn()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "Only supported on macOS"))
    }
}

/// ディレクトリに Spotlight インデックス対象外の設定を行う
///
/// 空の `.metadata_never_index` ファイルを作成することで、
/// macOS の Spotlight が大量の生成ファイルをスキャンするのを防ぐ。
pub fn prevent_spotlight_indexing(path: &Path) -> Result<(), std::io::Error> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    let flag_file = path.join(".metadata_never_index");
    if !flag_file.exists() {
        std::fs::write(flag_file, "")?;
        tracing::info!("🚫 Spotlight indexing disabled for: {}", path.display());
    }
    Ok(())
}

/// プロセスの優先度を上げる（Nice値の変更）
pub fn raise_process_priority() {
    // POSIX nice 値の変更は libc 等が必要だが、
    // ここでは概念的な実装に留めるか、Command で実行する。
    // 長時間稼働には標準的な優先度で十分な場合が多い。
}
