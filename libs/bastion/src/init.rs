/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

//! # Init - テンプレート生成モジュール
//!
//! セキュリティ関連のテンプレートファイルをプロジェクトに展開する。

use anyhow::{bail, Result};
use colored::*;
use std::fs;
use std::path::Path;
use crate::common::{self, ProjectType};

/// guardrails テンプレート（バイナリに埋め込み）
const GUARDRAILS_TEMPLATE: &str = include_str!("../templates/guardrails_template.rs");

/// secure_requirements テンプレート（バイナリに埋め込み）
const SECURE_REQUIREMENTS_TEMPLATE: &str = include_str!("../templates/secure_requirements.txt");

/// 指定された言語のテンプレートを生成する
pub fn run_init(language: &str) -> Result<()> {
    match language {
        "rust" => init_rust(),
        "python" => init_python(),
        "auto" => {
            println!("{}", "Detecting project type...".cyan());
            match common::detect_project_type() {
                ProjectType::Rust => init_rust(),
                ProjectType::Python => init_python(),
                ProjectType::Unknown => bail!("Could not auto-detect project type. Please specify 'rust' or 'python'."),
            }
        }
        _ => bail!(
            "Unknown language: '{}'. Supported: rust, python, auto",
            language
        ),
    }
}

fn init_rust() -> Result<()> {
    let target_path = "src/guardrails.rs";

    if Path::new(target_path).exists() {
        println!(
            "{} '{}' already exists. Skipping to avoid overwriting.",
            "Warning:".yellow().bold(),
            target_path
        );
        return Ok(());
    }

    if !Path::new("src").exists() {
        fs::create_dir_all("src")?;
    }

    fs::write(target_path, GUARDRAILS_TEMPLATE)?;

    println!("{} Generated '{}'", "✓".green().bold(), target_path);
    println!("");
    println!("  {} Add 'regex = \"1.10\"' to your Cargo.toml", "Next steps:".cyan().bold());
    println!("  Then use it in your code: 'mod guardrails; use guardrails::validate_input;'");

    Ok(())
}

fn init_python() -> Result<()> {
    let target_path = "secure_requirements.txt";

    if Path::new(target_path).exists() {
        println!(
            "{} '{}' already exists. Skipping to avoid overwriting.",
            "Warning:".yellow().bold(),
            target_path
        );
        return Ok(());
    }

    fs::write(target_path, SECURE_REQUIREMENTS_TEMPLATE)?;

    println!("{} Generated '{}'", "✓".green().bold(), target_path);
    println!("");
    println!("  {} Append to requirements.txt:", "Next steps:".cyan().bold());
    println!("  'cat secure_requirements.txt >> requirements.txt && pip install -r requirements.txt'");

    Ok(())
}
