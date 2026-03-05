/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

//! # Common - 共通ユーティリティモジュール
//!
//! プロジェクトの種類の判定や、パスの標準化など
//! 複数のモジュールで使用される共通ロジックを提供する。

use std::path::Path;

/// プロジェクトの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectType {
    Rust,
    Python,
    Unknown,
}

/// カレントディレクトリのファイル構成からプロジェクトの種類を判定する
pub fn detect_project_type() -> ProjectType {
    if Path::new("Cargo.toml").exists() {
        return ProjectType::Rust;
    }
    if Path::new("requirements.txt").exists() || Path::new("pyproject.toml").exists() {
        return ProjectType::Python;
    }
    ProjectType::Unknown
}

/// 指定されたパスが隠しファイル（ドットで始まる）または無視すべきディレクトリか判定する
pub fn is_ignored_path(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name.starts_with('.') || 
    matches!(name, "target" | "node_modules" | "venv" | ".venv" | "__pycache__" | "dist" | "build")
}
