/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, error, warn};
use std::fs;

pub struct SkillForge {
    template_dir: PathBuf,
    skills_output_dir: PathBuf,
}

impl SkillForge {
    pub fn new<P: AsRef<Path>>(template_dir: P, skills_output_dir: P) -> Self {
        Self {
            template_dir: template_dir.as_ref().to_path_buf(),
            skills_output_dir: skills_output_dir.as_ref().to_path_buf(),
        }
    }

    /// 新しいスキルを生成し、コンパイルする (自己修復ループ付き)
    pub async fn forge_skill(
        &self,
        skill_name: &str,
        rust_code: &str,
        retry_count: u32,
        description: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        let temp_dir = std::env::temp_dir().join(format!("skill_forge_{}_{}", skill_name, uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir)?;

        // 1. Copy Template
        Self::copy_dir(&self.template_dir, &temp_dir)?;

        // 2. Overwrite lib.rs (Securely)
        let lib_rs_path = temp_dir.join("src/lib.rs");
        fs::write(&lib_rs_path, rust_code)?;

        // Ensure no build.rs exists for security
        let build_rs = temp_dir.join("build.rs");
        if build_rs.exists() {
            fs::remove_file(build_rs)?;
        }

        // 3. Compile
        let _current_code = rust_code.to_string();
        for attempt in 0..=retry_count {
            info!("🛠️ [SkillForge] Compiling {} (Attempt {}/{})", skill_name, attempt + 1, retry_count + 1);
            
            let output = Command::new("cargo")
                .arg("build")
                .arg("--target")
                .arg("wasm32-wasip1")
                .arg("--release")
                .current_dir(&temp_dir)
                .stderr(Stdio::piped())
                .output()
                .await?;

            if output.status.success() {
                let wasm_file = temp_dir.join(format!("target/wasm32-wasip1/release/{}.wasm", "skill_generator")); // Template matches target name
                let final_path = self.skills_output_dir.join(format!("{}.wasm", skill_name));
                
                if !self.skills_output_dir.exists() {
                    fs::create_dir_all(&self.skills_output_dir)?;
                }
                
                fs::copy(&wasm_file, &final_path)?;
                info!("✅ [SkillForge] Successfully forged skill: {}", skill_name);
                
                // 4. Save Metadata (Phase 1: Capability-Aware Self-Wiring)
                let meta_path = self.skills_output_dir.join(format!("{}.meta.json", skill_name));
                let meta = crate::skills::SkillMetadata {
                    name: skill_name.to_string(),
                    description: description.to_string(),
                    capabilities: vec!["execute".to_string()], // Generated skills are mostly execution-based
                    inputs: vec!["String".to_string()],
                    outputs: vec!["String".to_string()],
                    allowed_hosts: vec![],
                };
                let meta_json = serde_json::to_string_pretty(&meta)?;
                fs::write(meta_path, meta_json)?;

                // Cleanup
                let _ = fs::remove_dir_all(&temp_dir);
                return Ok(final_path);
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                error!("❌ [SkillForge] Compilation failed for {}:\n{}", skill_name, stderr);
                
                if attempt < retry_count {
                    warn!("🔄 [SkillForge] Requesting self-healing for {}...", skill_name);
                    return Err(format!("Compilation failed. Stderr: {}", stderr).into());
                } else {
                    let _ = fs::remove_dir_all(&temp_dir);
                    return Err(format!("Compilation failed after {} attempts. Stderr: {}", retry_count + 1, stderr).into());
                }
            }
        }

        Err("Unexpected end of forge loop".into())
    }

    fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_dir() {
                let sub_dst = dst.join(entry.file_name());
                fs::create_dir_all(&sub_dst)?;
                Self::copy_dir(&entry.path(), &sub_dst)?;
            } else {
                fs::copy(entry.path(), dst.join(entry.file_name()))?;
            }
        }
        Ok(())
    }
}
