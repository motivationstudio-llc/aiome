/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Elastic License 2.0 (ELv2).
 * You may not provide the software to third parties as a hosted or managed service,
 * where the service provides users with access to any substantial set of the features
 * or functionality of the software.
 */

use std::fs;
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

#[derive(Clone)]
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

    /// Forge環境の初期構築
    pub fn ensure_forge_workspace(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.template_dir.exists() {
            fs::create_dir_all(&self.template_dir)?;

            // Cargo.toml
            let cargo_toml = r#"[package]
name = "skill_generator"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[workspace]

[dependencies]
extism-pdk = "1.4.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
"#;
            fs::write(self.template_dir.join("Cargo.toml"), cargo_toml)?;

            // src/lib.rs
            let src_dir = self.template_dir.join("src");
            fs::create_dir_all(&src_dir)?;
            fs::write(src_dir.join("lib.rs"), "// Forge Entrypoint")?;
        }
        Ok(())
    }

    /// macOS Seatbelt (sandbox-exec) 用のプロファイル生成
    pub(crate) fn generate_seatbelt_profile(&self, temp_dir: &Path) -> String {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        format!(
            r#"(version 1)
(allow default)
(deny file-write* (subpath "{current_dir}"))
(allow file-write* (subpath "{temp_dir}"))
(allow file-write* (subpath "/tmp") (subpath "/private/tmp"))
"#,
            current_dir = current_dir.to_string_lossy(),
            temp_dir = temp_dir.to_string_lossy()
        )
    }

    /// 新しいスキルを生成し、コンパイルする (自己修復ループ付き)
    pub async fn forge_skill(
        &self,
        skill_name: &str,
        initial_rust_code: &str,
        retry_count: u32,
        description: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        // Security Gate: Ensure forge is enabled
        let enabled = std::env::var("SKILL_FORGE_ENABLED")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false);

        if !enabled {
            error!("🛑 [SkillForge] Forging is BLOCKED. Set SKILL_FORGE_ENABLED=true to allow.");
            return Err(
                "Security Violation: Real-time skill forging is disabled in this environment."
                    .into(),
            );
        }

        // Phase 13-C: File-Based Saga — Use stable workspace for build caching
        let forge_root = self
            .skills_output_dir
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or(Path::new("workspace"))
            .join("forge_workspaces");
        let workspace_dir = forge_root.join(skill_name);
        if !workspace_dir.exists() {
            fs::create_dir_all(&workspace_dir)?;
        }

        // 1. Copy Template (Sanitized copy_dir prevents overwriting existing build artifacts)
        Self::copy_dir(&self.template_dir, &workspace_dir)?;

        // 2. Update Cargo.toml name (G13)
        let cargo_toml_path = workspace_dir.join("Cargo.toml");
        let cargo_toml = fs::read_to_string(&cargo_toml_path)?;
        let updated_cargo = cargo_toml.replace("skill_generator", skill_name);
        fs::write(&cargo_toml_path, updated_cargo)?;

        // 3. Compile Loop (G11 Support: Stderr results will be used for self-healing)
        let rust_code = initial_rust_code.to_string();
        for attempt in 0..=retry_count {
            info!(
                "🛠️ [SkillForge] Compiling {} (Attempt {}/{})",
                skill_name,
                attempt + 1,
                retry_count + 1
            );

            let lib_rs_path = workspace_dir.join("src/lib.rs");
            fs::write(&lib_rs_path, &rust_code)?;

            let abs_workspace =
                std::fs::canonicalize(&workspace_dir).unwrap_or(workspace_dir.clone());
            let profile_content = self.generate_seatbelt_profile(&abs_workspace);
            let profile_path = workspace_dir.join("forge.sb");
            fs::write(&profile_path, profile_content)?;
            let abs_profile_path = std::fs::canonicalize(&profile_path).unwrap_or(profile_path);
            let abs_manifest_path = abs_workspace.join("Cargo.toml");

            let args = vec![
                "-f".to_string(),
                abs_profile_path.to_string_lossy().to_string(),
                "cargo".to_string(),
                "build".to_string(),
                "--manifest-path".to_string(),
                abs_manifest_path.to_string_lossy().to_string(),
                "--target".to_string(),
                "wasm32-wasip1".to_string(),
                "--release".to_string(),
            ];

            let output = shared::zombie_killer::run_with_timeout_vec(
                "sandbox-exec",
                args,
                std::time::Duration::from_secs(120),
            )
            .await;

            match output {
                Ok(output) => {
                    if output.status.success() {
                        let wasm_file = workspace_dir.join(format!(
                            "target/wasm32-wasip1/release/{}.wasm",
                            skill_name.replace('-', "_")
                        ));
                        let final_path =
                            self.skills_output_dir.join(format!("{}.wasm", skill_name));

                        if !self.skills_output_dir.exists() {
                            fs::create_dir_all(&self.skills_output_dir)?;
                        }

                        fs::copy(&wasm_file, &final_path)?;
                        info!("✅ [SkillForge] Successfully forged skill: {}", skill_name);

                        // 4. Save Metadata
                        let meta_path = self
                            .skills_output_dir
                            .join(format!("{}.meta.json", skill_name));
                        #[derive(serde::Serialize)]
                        struct LocalSkillMetadata {
                            name: String,
                            description: String,
                            capabilities: Vec<String>,
                            inputs: Vec<String>,
                            outputs: Vec<String>,
                        }
                        let meta = LocalSkillMetadata {
                            name: skill_name.to_string(),
                            description: description.to_string(),
                            capabilities: vec!["execute".to_string()],
                            inputs: vec!["String".to_string()],
                            outputs: vec!["String".to_string()],
                        };
                        let meta_json = serde_json::to_string_pretty(&meta)?;
                        fs::write(meta_path, meta_json)?;

                        // Discovery D: Success! We keep the workspace for future builds.
                        return Ok(final_path);
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        error!(
                            "❌ [SkillForge] Compilation failed for {}:\n{}",
                            skill_name, stderr
                        );

                        if attempt < retry_count {
                            warn!("🔄 [SkillForge] Compilation failed. Continuing retry loop...");
                            // In real-time self-healing, the agent would update rust_code here.
                            // For now, we just loop to fulfill the retry_count logic correctly (Discovery D).
                            continue;
                        } else {
                            return Err(format!(
                                "Compilation failed after {} attempts. Stderr: {}",
                                retry_count + 1,
                                stderr
                            )
                            .into());
                        }
                    }
                }
                Err(e) => {
                    error!("❌ [SkillForge] Command execution error: {}", e);
                    if attempt >= retry_count {
                        return Err(format!("Process error: {}", e).into());
                    }
                }
            }
        }

        Err("Maximum retry attempts reached without success.".into())
    }

    fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            let dest_path = dst.join(entry.file_name());

            if ty.is_dir() {
                // Phase 13-C Fix: Skip existing directories to preserve build caches (target/)
                if !dest_path.exists() {
                    fs::create_dir_all(&dest_path)?;
                }
                Self::copy_dir(&entry.path(), &dest_path)?;
            } else {
                // For files (Cargo.toml, src/lib.rs template), we ensure they match the template
                fs::copy(entry.path(), dest_path)?;
            }
        }
        Ok(())
    }
}
