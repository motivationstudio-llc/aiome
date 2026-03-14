/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

use crate::skills::importer::{L1Metadata, L2Metadata, L3Metadata, SkillManifest};
use serde::Deserialize;
use tracing::{info, warn};

#[derive(Deserialize, Debug)]
struct GitHubActionYaml {
    name: String,
    description: String,
    inputs: Option<serde_yaml::Value>,
    runs: GitHubActionRuns,
}

#[derive(Deserialize, Debug)]
struct GitHubActionRuns {
    using: String,
    main: Option<String>,
    steps: Option<serde_yaml::Value>,
}

pub struct ActionsImporter;

impl ActionsImporter {
    /// [A-3] Vampire Attack: GitHub Actions Importer
    /// Absorbs standard action.yml files into Aiome Progressive Disclosure Skills.
    pub fn parse_action_yml(content: &str) -> Option<SkillManifest> {
        let parsed: GitHubActionYaml = match serde_yaml::from_str(content) {
            Ok(p) => p,
            Err(e) => {
                warn!("⚠️ [ActionsImporter] Failed to parse action.yml: {}", e);
                return None;
            }
        };

        info!(
            "🐙 [ActionsImporter] Absorbing GitHub Action: {}",
            parsed.name
        );

        let engine_type = if parsed.runs.using.starts_with("node") {
            "node" // E.g., node20
        } else if parsed.runs.using == "docker" {
            "docker"
        } else {
            "composite" // Or bash script
        };

        Some(SkillManifest {
            l1: L1Metadata {
                name: parsed.name.replace(' ', "_").to_lowercase(),
                trigger_description: format!("(From GitHub Action) {}", parsed.description),
            },
            l2: L2Metadata {
                extended_description: parsed.description,
                inputs_schema: serde_json::to_value(&parsed.inputs)
                    .unwrap_or(serde_json::json!({})),
                examples: vec![],
            },
            l3: L3Metadata {
                engine: engine_type.to_string(),
                entry_point: parsed.runs.main.unwrap_or_else(|| "index.js".to_string()),
                source_code: None, // Source is usually separate in GH Actions
            },
        })
    }
}
