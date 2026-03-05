/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

//! # Scanner - 脆弱性スキャンモジュール
//!
//! プロジェクトの脆弱性スキャン・シークレット検出を行う。

use anyhow::Result;
use colored::*;
use regex::Regex;
use std::fs;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

use crate::common::{self, ProjectType};
use crate::python_check;

/// メインのスキャン処理を実行する
pub fn run_scan() -> Result<()> {
    println!("{}", "=== BASTION SECURITY CHECK START ===".bold().cyan());

    let project_type = common::detect_project_type();
    
    match project_type {
        ProjectType::Rust => {
            println!("{}", "[+] Rust Project Detected".green());
            run_rust_checks()?;
        }
        ProjectType::Python => {
            println!("{}", "[+] Python Project Detected".green());
            run_python_checks()?;
            if Path::new("requirements.txt").exists() {
                python_check::check_secure_requirements("requirements.txt")?;
            }
        }
        ProjectType::Unknown => {
            println!("{}", "[!] Generic Project / Unknown Language".yellow());
        }
    }

    println!("{}", "
[+] Starting Secret Scan...".yellow());
    scan_for_secrets(".")?;

    println!("{}", "
=== CHECK FINISHED ===".bold().cyan());
    Ok(())
}

fn run_rust_checks() -> Result<()> {
    println!("Running cargo audit...");
    if Command::new("cargo").args(["audit"]).status().is_err() {
        println!("{}", "Warning: 'cargo-audit' not found. Skip.".red());
    }

    println!("Running cargo clippy...");
    Command::new("cargo").args(["clippy", "--", "-D", "warnings"]).status()?;
    Ok(())
}

fn run_python_checks() -> Result<()> {
    println!("Running pip-audit...");
    if Command::new("pip-audit").status().is_err() {
        println!("{}", "Warning: 'pip-audit' not found. Skip.".red());
    }

    println!("Running bandit...");
    if Command::new("bandit").args(["-r", "."]).status().is_err() {
        println!("{}", "Warning: 'bandit' not found. Skip.".red());
    }
    Ok(())
}

fn scan_for_secrets(dir: &str) -> Result<()> {
    // 改善されたシークレット検出用正規表現（誤検知を減らすために境界を意識）
    let re = Regex::new(
        r#"(?i)(api_key|password|secret|token|private_key|access_key|auth_token)\s*[:=]\s*['""]([a-zA-Z0-9_\-]{12,})['""]"#,
    ).unwrap();

    let walker = WalkDir::new(dir).into_iter();

    for entry in walker.filter_entry(|e| !common::is_ignored_path(e.path())) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let path = entry.path();
            if is_scannable_file(path) {
                check_file_content(path, &re)?;
            }
        }
    }
    Ok(())
}

fn is_scannable_file(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|ext| matches!(ext, "rs" | "py" | "js" | "ts" | "env" | "json" | "toml" | "yaml" | "yml" | "md"))
        .unwrap_or(false)
}

fn check_file_content(path: &Path, re: &Regex) -> Result<()> {
    if let Ok(content) = fs::read_to_string(path) {
        for (i, line) in content.lines().enumerate() {
            if re.is_match(line) {
                println!(
                    "{} Found potential secret in {:?}:{} -> {}",
                    "[ALERT]".red().bold(),
                    path,
                    i + 1,
                    line.trim()
                );
            }
        }
    }
    Ok(())
}
