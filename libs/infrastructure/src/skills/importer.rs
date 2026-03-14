/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

use serde::{Deserialize, Serialize};

/// [A-3] Progressive Disclosure Architecture
/// Separation of metadata into layers to optimize token consumption.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkillManifest {
    pub l1: L1Metadata, // Trigger level (Minimal)
    pub l2: L2Metadata, // Description level (Context)
    pub l3: L3Metadata, // Execution level (Code/Wasm)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct L1Metadata {
    pub name: String,
    pub trigger_description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct L2Metadata {
    pub extended_description: String,
    pub inputs_schema: serde_json::Value,
    pub examples: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct L3Metadata {
    pub engine: String, // "wasm", "docker", "mcp"
    pub entry_point: String,
    pub source_code: Option<String>,
}

pub struct SkillImporter;

impl SkillImporter {
    /// Claude Code (.md) 形式のスキルをインポート
    pub fn parse_skill_md(content: &str) -> Option<SkillManifest> {
        // Simple Markdown parser for headers and code blocks
        let mut name = "unnamed".to_string();
        let mut description = "".to_string();
        let mut code = "".to_string();

        let mut in_code_block = false;
        for line in content.lines() {
            if line.starts_with("# ") && name == "unnamed" {
                name = line[2..].trim().to_string();
            } else if line.starts_with("```") {
                in_code_block = !in_code_block;
            } else if in_code_block {
                code.push_str(line);
                code.push('\n');
            } else if !line.trim().is_empty() && description.is_empty() {
                description = line.trim().to_string();
            }
        }

        Some(SkillManifest {
            l1: L1Metadata {
                name: name.clone(),
                trigger_description: description.clone(),
            },
            l2: L2Metadata {
                extended_description: content.to_string(),
                inputs_schema: serde_json::json!({"type": "string"}),
                examples: vec![],
            },
            l3: L3Metadata {
                engine: "script".to_string(),
                entry_point: name,
                source_code: Some(code),
            },
        })
    }

    /// Agency-Agents (YAML) 形式のスキルをインポート
    pub fn parse_agency_yaml(content: &str) -> Option<SkillManifest> {
        #[derive(Deserialize)]
        struct AgencyYaml {
            name: String,
            description: String,
            #[serde(default)]
            instructions: String,
        }

        let parsed: AgencyYaml = serde_yaml::from_str(content).ok()?;

        Some(SkillManifest {
            l1: L1Metadata {
                name: parsed.name.clone(),
                trigger_description: parsed.description.clone(),
            },
            l2: L2Metadata {
                extended_description: parsed.instructions,
                inputs_schema: serde_json::json!({"type": "object"}),
                examples: vec![],
            },
            l3: L3Metadata {
                engine: "mega-skill".to_string(), // Requires SkillCreator for actual code
                entry_point: parsed.name,
                source_code: None,
            },
        })
    }

    /// OpenAPI スキーマからスキルを生成
    pub fn parse_openapi(content: &str) -> Vec<SkillManifest> {
        // Simplified OpenAPI parsing logic
        let mut skills = Vec::new();
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(paths) = val.get("paths").and_then(|p| p.as_object()) {
                for (path, methods) in paths {
                    if let Some(methods_obj) = methods.as_object() {
                        for (method, details) in methods_obj {
                            let operation_id = details
                                .get("operationId")
                                .and_then(|v| v.as_str())
                                .unwrap_or(path);
                            let summary = details
                                .get("summary")
                                .and_then(|v| v.as_str())
                                .unwrap_or("No description");

                            skills.push(SkillManifest {
                                l1: L1Metadata {
                                    name: operation_id.to_string(),
                                    trigger_description: summary.to_string(),
                                },
                                l2: L2Metadata {
                                    extended_description: format!(
                                        "Endpoint: {} {}",
                                        method.to_uppercase(),
                                        path
                                    ),
                                    inputs_schema: details
                                        .get("parameters")
                                        .cloned()
                                        .unwrap_or(serde_json::json!([])),
                                    examples: vec![],
                                },
                                l3: L3Metadata {
                                    engine: "api".to_string(),
                                    entry_point: path.clone(),
                                    source_code: None,
                                },
                            });
                        }
                    }
                }
            }
        }
        skills
    }
}
