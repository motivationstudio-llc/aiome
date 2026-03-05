/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

//! # Python Check - 推奨ライブラリ検証モジュール
//!
//! Pythonプロジェクトの requirements.txt をチェックし、
//! セキュリティ上推奨されるライブラリが含まれているかを検証する。

use anyhow::Result;
use colored::*;
use std::fs;

/// 推奨するセキュリティライブラリのリスト
const RECOMMENDED_PACKAGES: &[(&str, &str)] = &[
    ("defusedxml", "XML処理の安全化（XXE攻撃対策）"),
    ("bandit", "静的解析によるセキュリティ脆弱性検出"),
    ("pip-audit", "依存関係の脆弱性チェック"),
];

/// requirements.txt を読み込み、推奨ライブラリの有無をチェックする
pub fn check_secure_requirements(requirements_path: &str) -> Result<()> {
    println!(
        "
{}",
        "[+] Checking recommended security packages...".yellow()
    );

    let content = fs::read_to_string(requirements_path)?;
    let content_lower = content.to_lowercase();

    let mut missing_count = 0;

    for (package, description) in RECOMMENDED_PACKAGES {
        if content_lower.contains(&package.to_lowercase()) {
            println!("  {} {} is present", "✓".green().bold(), package);
        } else {
            println!(
                "  {} {} is missing — {}",
                "✗".red().bold(),
                package,
                description
            );
            missing_count += 1;
        }
    }

    if missing_count > 0 {
        println!(
            "
  {} Run '{}' to generate recommended requirements.",
            "TIP:".cyan().bold(),
            "bastion init python".bold()
        );
    } else {
        println!(
            "  {}",
            "All recommended security packages are present!".green()
        );
    }

    Ok(())
}
