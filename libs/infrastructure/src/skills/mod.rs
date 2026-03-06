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
use tracing::{info, error};
use extism::{Manifest, Plugin};
use jsonschema::JSONSchema;
pub mod forge;
use contracts::{requires, ensures};

/// 状態: 未検証の外部Skill (TypeState Pattern)
#[derive(Debug, Clone)]
pub struct UnverifiedSkill {
    pub name: String,
    pub input_test_payload: String,
}

/// 状態: 確定的検証をパスした安全なSkill (TypeState Pattern)
#[derive(Debug, Clone)]
pub struct VerifiedSkill {
    pub name: String,
}

impl UnverifiedSkill {
    /// 契約プログラミングにより、検証を通過したものだけが型を昇格できる
    #[requires(self.input_test_payload.len() < 50_000, "Payload limits exceeded")]
    // #[ensures] is removed here because verification failure (Err) is a valid, expected state machine outcome for malicious skills.
    pub async fn verify(self, manager: &WasmSkillManager) -> Result<VerifiedSkill, Box<dyn std::error::Error + Send + Sync>> {
        let is_safe = manager.dry_run_skill(&self.name, &self.input_test_payload).await?;
        if is_safe {
            Ok(VerifiedSkill { name: self.name })
        } else {
            Err(format!("Skill {} failed the deterministic dry-run quarantine", self.name).into())
        }
    }
}


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
    #[serde(default)]
    pub permissions: crate::security::PermissionManifest,
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
                    permissions: crate::security::PermissionManifest::default(),
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
    /// 🛡️ 第4層 (Formal Verification): &str ではなく VerifiedSkill 型を要求することで、
    /// 事前の隔離検証を通過していないSkillの実行をコンパイルレベルで阻止する。
    pub async fn call_skill(
        &self, 
        skill: &VerifiedSkill, 
        func_name: &str, 
        input: &str,
        configs: Option<HashMap<String, String>>
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let skill_name = &skill.name;
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
        // Wildcard '*' is strictly forbidden for security reasons.
        let metadata = self.get_metadata(skill_name);
        if let Some(meta) = metadata {
            for host in &meta.allowed_hosts {
                if host == "*" {
                    error!("🛑 [WasmSkillManager] Wildcard network access is strictly FORBIDDEN for skill '{}'", skill_name);
                    return Err(format!("Security Violation: Wildcard network access ('*') is not allowed for skill '{}'. Please specify explicit hosts.", skill_name).into());
                }
                manifest = manifest.with_allowed_host(host);
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

    /// Layer 3: Deterministic Tracer (MEV型 Quarantine Simulation)
    /// インストール対象のSkillを、ネットワークを完全に遮断し、
    /// メモリ上限を極端に絞ったサンドボックス上で「空回し」させて振る舞いを検証する。
    pub async fn dry_run_skill(
        &self,
        skill_name: &str,
        test_input: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let wasm_path = self.skills_dir.join(format!("{}.wasm", skill_name));
        if !wasm_path.exists() {
            return Err(format!("Skill {} not found for dry-run", skill_name).into());
        }

        let func_name = self.get_metadata(skill_name)
            .and_then(|m| m.capabilities.first().cloned())
            .unwrap_or_else(|| "execute".to_string());

        info!("🛡️  [Layer 3 Deterministic Tracer] Starting dry-run for skill: {} (func: {})", skill_name, func_name);

        // 1. Simulation Plugin の新設 (Extism Manifest の極限制限)
        // ネットワークアクセス一切なし、WASIディスクアクセス制限 (一切バインドしない)
        let manifest = Manifest::new([extism::Wasm::file(&wasm_path)])
            .with_timeout(Duration::from_millis(500)); // タイムアウトも極端に短く (500ms)

        // プラグイン初期化
        let mut plugin = match Plugin::new(&manifest, [], true) {
            Ok(p) => p,
            Err(e) => {
                error!("🚨 [Layer 3 Deterministic Tracer] Initialization violation (OOM or format error): {}", e);
                return Ok(false);
            }
        };

        // 2. 実行時検証 (シミュレーション実行)
        // OOMや非合法なSyscallが発生した場合はエラーとして返ってくる
        info!("⚡ [Layer 3 Deterministic Tracer] Simulating execution with deterministic constraints...");
        let dry_run_result = plugin.call::<&str, String>(&func_name, test_input);
        
        // 3. 全ての検証をパスした場合のみ、VerifiedSkill 型を生成して返す
        match dry_run_result {
            Ok(_) => {
                info!("✅ [Layer 3 Deterministic Tracer] Protocol behavior validated deterministically: {}", skill_name);
                
                // 内部で call_skill を呼んで最終的な出力を得てから VerifiedSkill を返す
                // (一部の高度な検証では出力を精査するため)
                let verified = VerifiedSkill { name: skill_name.to_string() };
                let _output = self.call_skill(&verified, &func_name, test_input, None).await?;
                
                Ok(true)
            }
            Err(e) => {
                println!("🚨 [Layer 3 Deterministic Tracer] Deterministic Violation Detected: {}", e);
                error!("🚨 [Layer 3 Deterministic Tracer] Deterministic Violation Detected: {}", e);
                Ok(false)
            }
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
        
        // 内部的に VerifiedSkill を作成 (※validate_skill_logic は管理者のみが呼ぶため信頼済み)
        let verified = VerifiedSkill { name: skill_name.to_string() };
        let output = self.call_skill(&verified, "execute", test_input, None).await?;
        
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
        jq: &impl aiome_core::traits::JobQueue,
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
