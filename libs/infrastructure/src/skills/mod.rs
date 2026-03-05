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
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, error, warn};
use extism::{Manifest, Plugin};
use jsonschema::JSONSchema;
pub mod forge;


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
}

pub struct WasmSkillManager {
    skills_dir: PathBuf,
    memory_limit_bytes: u64,
    timeout: Duration,
}

impl WasmSkillManager {
    pub fn new<P: AsRef<Path>>(skills_dir: P, _allowed_root: P) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let skills_dir = skills_dir.as_ref().to_path_buf();
        if !skills_dir.exists() {
            std::fs::create_dir_all(&skills_dir)?;
        }
        Ok(Self { 
            skills_dir, 
            memory_limit_bytes: 10 * 1024 * 1024, // 10MB default
            timeout: Duration::from_millis(5000), // 5s default
        })
    }

    pub fn with_limits(mut self, memory_bytes: u64, timeout: Duration) -> Self {
        self.memory_limit_bytes = memory_bytes;
        self.timeout = timeout;
        self
    }

    /// 全スキルのメタデータを一覧取得する (Self-Wiring 用)
    pub fn list_skills_with_metadata(&self) -> Vec<SkillMetadata> {
        let mut list = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.skills_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("meta.json") {
                    if let Ok(data) = std::fs::read_to_string(&path) {
                        if let Ok(meta) = serde_json::from_str::<SkillMetadata>(&data) {
                            list.push(meta);
                        }
                    }
                }
            }
        }
        
        // メタデータがないスキルについては、ファイル名から最小限のものを生成
        let all_wasm = self.list_skills();
        for name in all_wasm {
            if !list.iter().any(|m| m.name == name) {
                list.push(SkillMetadata {
                    name: name.clone(),
                    description: "No metadata provided".to_string(),
                    capabilities: vec!["execute".to_string()],
                    inputs: vec!["String".to_string()],
                    outputs: vec!["String".to_string()],
                    allowed_hosts: vec![],
                });
            }
        }
        list
    }

    /// 利用可能なスキル名を一覧表示する
    pub fn list_skills(&self) -> Vec<String> {
        let mut skills = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.skills_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("wasm") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        skills.push(name.to_string());
                    }
                }
            }
        }
        skills
    }

    /// WASMスキルを実行する (シークレット注入対応)
    pub async fn call_skill(
        &self, 
        skill_name: &str, 
        func_name: &str, 
        input: &str,
        configs: Option<HashMap<String, String>>
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let wasm_path = self.skills_dir.join(format!("{}.wasm", skill_name));
        if !wasm_path.exists() {
            return Err(format!("Skill {} not found", skill_name).into());
        }

        // 厳密なサンドボックス設定
        let mut manifest = Manifest::new([extism::Wasm::file(&wasm_path)]);
        
        // 1. WASI Jail Bindings: Sandboxのルートのみをバインド
        if let Some(parent) = self.skills_dir.parent() {
            let jail_root = std::fs::canonicalize(parent)?;
            manifest = manifest.with_allowed_path(jail_root.to_string_lossy().to_string(), "/mnt");
        }

        // 2. Network: Whitelist-based isolation (Zero Trust)
        let metadata = self.get_metadata(skill_name);
        if let Some(meta) = metadata {
            if meta.allowed_hosts.is_empty() {
                 // Default: No network if not specified
            } else if meta.allowed_hosts.contains(&"*".to_string()) {
                warn!("⚠️ [WasmSkillManager] Skill '{}' uses wildcard network access. Be careful.", skill_name);
                manifest = manifest.with_allowed_host("*");
            } else {
                for host in &meta.allowed_hosts {
                    manifest = manifest.with_allowed_host(host);
                }
            }
        }

        // 3. Resource Limits & Timeouts
        manifest = manifest.with_timeout(self.timeout);

        // 3. Credential Injection via Config memory
        if let Some(cfg) = configs {
            for (key, value) in cfg {
                manifest = manifest.with_config(std::iter::once((key, value)));
            }
        }

        // プラグインの初期化と実行
        info!("🚀 [WasmSkillManager] Initializing WASM plugin: {}", skill_name);
        let mut plugin = Plugin::new(&manifest, [], true)
            .map_err(|e| format!("Failed to initialize WASM plugin {}: {}", skill_name, e))?;
        
        info!("⚡ [WasmSkillManager] Calling function: {}::{}", skill_name, func_name);
        
        let result = plugin.call::<&str, String>(func_name, input)
            .map_err(|e| {
                error!("❌ [WasmSkillManager] Skill execution failed for {}: {}", skill_name, e);
                if e.to_string().to_lowercase().contains("timeout") {
                    format!("WASM execution timed out after {:?}", self.timeout)
                } else {
                    format!("WASM execution error: {}", e)
                }
            })?;

        info!("✅ [WasmSkillManager] Skill execution successful: {}", skill_name);
        Ok(result)
    }

    pub fn get_metadata(&self, skill_name: &str) -> Option<SkillMetadata> {
        let meta_path = self.skills_dir.join(format!("{}.meta.json", skill_name));
        if let Ok(data) = std::fs::read_to_string(meta_path) {
            serde_json::from_str(&data).ok()
        } else {
            None
        }
    }

    /// ドライラン（Dry-Run）による論理検証。
    /// 指定されたテスト入力に対して、期待されるスキーマに合致するかチェックする。
    pub async fn validate_skill_logic(
        &self, 
        skill_name: &str, 
        test_input: &str,
        expected_schema_json: &str
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        info!("🧪 [WasmSkillManager] Validating skill logic for: {}", skill_name);
        
        let output = self.call_skill(skill_name, "execute", test_input, None).await?;
        
        // JSON Schema validation
        let schema_val: serde_json::Value = serde_json::from_str(expected_schema_json)?;
        let instance: serde_json::Value = serde_json::from_str(&output)?;
        
        let compiled = JSONSchema::compile(&schema_val)
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Schema compilation failed: {}", e))) as Box<dyn std::error::Error + Send + Sync>)?;

        if let Err(mut errors) = compiled.validate(&instance) {
            let first_error = errors.next().map(|e| e.to_string()).unwrap_or_else(|| "Unknown validation error".to_string());
            error!("❌ [WasmSkillManager] Logic validation failed for {}: {}", skill_name, first_error);
            return Ok(false);
        }

        info!("✅ [WasmSkillManager] Logic validation successful: {}", skill_name);
        Ok(true)
    }

    /// 知識ベース（Karma）から最適なスキルを意味的に探索する (Self-Wiring Capability)
    pub async fn search_skill_in_knowledge(
        &self,
        query: &str,
        jq: &impl factory_core::traits::JobQueue,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        // 現在のスキル一覧を取得
        let available_skills = self.list_skills();
        if available_skills.is_empty() {
            return Ok(None);
        }

        // Karmaから類似したレッスンを検索 (Top 5)
        let entries = jq.fetch_relevant_karma(query, "global", 5, "current").await?;
        
        for entry in entries {
            // エントリ内にスキル名が含まれているか、あるいはスキル名そのものが関連しているかチェック
            for skill in &available_skills {
                if entry.to_lowercase().contains(&skill.to_lowercase()) {
                    info!("🧠 [Self-Wiring] Found relevant skill '{}' via knowledge: {}", skill, entry);
                    return Ok(Some(skill.clone()));
                }
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests;
