/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use std::path::{PathBuf};
use crate::skills::importer::SkillManifest;
use crate::skills::forge::SkillForge;
use tracing::info;

/// [A-3] Cleanroom Environment
/// A strictly isolated environment for building and testing skills before
/// they are allowed to touch the main Aiome instance.
pub struct Cleanroom {
    forge: SkillForge,
    workspace: PathBuf,
}

impl Cleanroom {
    pub fn new(forge: SkillForge, workspace: PathBuf) -> Self {
        Self { forge, workspace }
    }

    /// [Vampire Attack] Process an imported manifest and attempt to forge it.
    pub async fn process_import(&self, manifest: SkillManifest) -> anyhow::Result<PathBuf> {
        info!("🧪 [Cleanroom] Processing import for skill: {}", manifest.l1.name);

        match manifest.l3.engine.as_str() {
            "script" => {
                if let Some(source) = manifest.l3.source_code {
                    info!("🛠️ [Cleanroom] Script detected. Attempting to forge into Wasm...");
                    // Try to forge the script into a Wasm skill using the existing Forge
                    let path = self.forge.forge_skill(
                        &manifest.l1.name,
                        &source,
                        3, // Retries
                        &manifest.l1.trigger_description
                    ).await.map_err(|e| anyhow::anyhow!("Forge failed: {}", e))?;
                    
                    return Ok(path);
                }
                Err(anyhow::anyhow!("No source code provided for script import"))
            }
            "api" => {
                info!("🌐 [Cleanroom] API identified. Generating bridge skill...");
                // In production, this would generate Rust code that calls the OpenAPI endpoint
                let bridge_code = format!(
                    "// Generated bridge for {}\nfn execute() {{ unimplemented!(); }}",
                    manifest.l3.entry_point
                );
                let path = self.forge.forge_skill(
                    &manifest.l1.name,
                    &bridge_code,
                    1,
                    &manifest.l1.trigger_description
                ).await.map_err(|e| anyhow::anyhow!("Forge failed: {}", e))?;
                Ok(path)
            }
            _ => Err(anyhow::anyhow!("Unsupported L3 engine: {}", manifest.l3.engine)),
        }
    }
}
