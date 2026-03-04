use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, error};
use extism::{Manifest, Plugin};
pub mod forge;


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
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

        // 2. Network: Allow all hosts for now (controlled by Intent Parser)
        manifest = manifest.with_allowed_host("*");

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
