/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

use serde::{Deserialize, Serialize};

/// ShortsFactory 全体の設定
#[derive(Clone, Serialize, Deserialize)]
pub struct FactoryConfig {
    /// Ollama API エンドポイント
    pub ollama_url: String,
    /// ComfyUI REST/WebSocket API エンドポイント
    pub comfyui_api_url: String,
    /// バッチサイズ（一括企画する動画の本数）
    pub batch_size: usize,
    /// ComfyUI タイムアウト（秒）
    pub comfyui_timeout_secs: u64,
    /// 本番用モデル名
    pub model_name: String,
    /// 台本生成用モデル名 (Gemini等)
    pub script_model: String,
    /// ComfyUI のベースディレクトリ (Zero-Copy)
    pub comfyui_base_dir: String,
    /// Brave Search API Key for The Automaton's Brain (Phase 10-B)
    pub brave_api_key: String,
    /// 最終動画の納品先ディレクトリ (Phase 10-C)
    pub export_dir: String,
    /// プロジェクトのワークスペースディレクトリ (Phase 10-D)
    pub workspace_dir: String,
    /// ファイル清掃までの経過時間(時間) (Phase 10-D)
    pub clean_after_hours: u64,
    /// SNS API Key (e.g. YouTube Data API Key)
    pub sns_api_key: String,
    /// Gemini API Key for The Oracle (Phase 11-D)
    pub gemini_api_key: String,
    /// TikTok API Key for Phase 11 Sentinel (Placeholder)
    pub tiktok_api_key: String,
    /// Unleashed Mode (Platinum Edition): Bypass all level requirements
    pub unleashed_mode: bool,
    /// Primary Artifact Extension (e.g. .mp4, .png)
    pub artifact_extension: String,
}

impl std::fmt::Debug for FactoryConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FactoryConfig")
            .field("ollama_url", &self.ollama_url)
            .field("comfyui_api_url", &self.comfyui_api_url)
            .field("batch_size", &self.batch_size)
            .field("comfyui_timeout_secs", &self.comfyui_timeout_secs)
            .field("model_name", &self.model_name)
            .field("comfyui_base_dir", &self.comfyui_base_dir)
            .field("brave_api_key", if self.brave_api_key.is_empty() { &"" } else { &"***" })
            .field("export_dir", &self.export_dir)
            .field("workspace_dir", &self.workspace_dir)
            .field("clean_after_hours", &self.clean_after_hours)
            .field("sns_api_key", if self.sns_api_key.is_empty() { &"" } else { &"***" })
            .field("gemini_api_key", if self.gemini_api_key.is_empty() { &"" } else { &"***" })
            .field("tiktok_api_key", if self.tiktok_api_key.is_empty() { &"" } else { &"***" })
            .field("unleashed_mode", &self.unleashed_mode)
            .field("artifact_extension", &self.artifact_extension)
            .finish()
    }
}

impl FactoryConfig {
    /// 設定をファイルまたは環境変数から読み込む
    pub fn load() -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            // デフォルト値の設定
            .set_default("ollama_url", "http://localhost:11434/v1")?
            .set_default("comfyui_api_url", std::env::var("COMFYUI_API_URL").unwrap_or_else(|_| "ws://127.0.0.1:8188/ws".to_string()))?
            .set_default("batch_size", 10)?
            .set_default("comfyui_timeout_secs", 180)?
            .set_default("model_name", "qwen2.5-coder:32b")?
            .set_default("script_model", "gemini-2.0-flash")?
            .set_default("comfyui_base_dir", std::env::var("COMFYUI_BASE_DIR").unwrap_or_else(|_| "/Users/motista/Desktop/ComfyUI".to_string()))?
            .set_default("brave_api_key", std::env::var("BRAVE_API_KEY").unwrap_or_else(|_| "".to_string()))?
            .set_default("export_dir", std::env::var("EXPORT_DIR").unwrap_or_else(|_| "/Users/motista/Library/Mobile Documents/com~apple~CloudDocs/Aiome_Exports".to_string()))?
            .set_default("workspace_dir", std::env::var("WORKSPACE_DIR").unwrap_or_else(|_| "./workspace".to_string()))?
            .set_default("clean_after_hours", 24)?
            .set_default("sns_api_key", std::env::var("SNS_API_KEY").or_else(|_| std::env::var("YOUTUBE_API_KEY")).unwrap_or_else(|_| "".to_string()))?
            .set_default("gemini_api_key", std::env::var("GEMINI_API_KEY").unwrap_or_else(|_| "".to_string()))?
            .set_default("tiktok_api_key", std::env::var("TIKTOK_API_KEY").unwrap_or_else(|_| "".to_string()))?
            .set_default("unleashed_mode", std::env::var("UNLEASHED_MODE").map(|v| v.to_lowercase() == "true").unwrap_or(false))?
            .set_default("artifact_extension", ".mp4")?
            // config.toml があれば読み込む
            .add_source(config::File::with_name("config").required(false))
            // 環境変数 (SHORTS_FACTORY_*) があれば上書き
            .add_source(config::Environment::with_prefix("SHORTS_FACTORY"))
            .build()?;

        settings.try_deserialize()
    }
}

impl Default for FactoryConfig {
    fn default() -> Self {
        Self::load().unwrap_or_else(|_| {
            Self {
                ollama_url: "http://localhost:11434/v1".to_string(),
                comfyui_api_url: std::env::var("COMFYUI_API_URL").unwrap_or_else(|_| "ws://127.0.0.1:8188/ws".to_string()),
                batch_size: 10,
                comfyui_timeout_secs: 180,
                model_name: "qwen2.5-coder:32b".to_string(),
                script_model: "gemini-2.0-flash".to_string(),
                comfyui_base_dir: std::env::var("COMFYUI_BASE_DIR").unwrap_or_else(|_| "/Users/motista/Desktop/ComfyUI".to_string()),
                brave_api_key: std::env::var("BRAVE_API_KEY").unwrap_or_else(|_| "".to_string()),
                export_dir: std::env::var("EXPORT_DIR").unwrap_or_else(|_| "/Users/motista/Library/Mobile Documents/com~apple~CloudDocs/Aiome_Exports".to_string()),
                workspace_dir: std::env::var("WORKSPACE_DIR").unwrap_or_else(|_| "./workspace".to_string()),
                clean_after_hours: 24,
                sns_api_key: std::env::var("SNS_API_KEY").or_else(|_| std::env::var("YOUTUBE_API_KEY")).unwrap_or_else(|_| "".to_string()),
                gemini_api_key: std::env::var("GEMINI_API_KEY").unwrap_or_else(|_| "".to_string()),
                tiktok_api_key: std::env::var("TIKTOK_API_KEY").unwrap_or_else(|_| "".to_string()),
                unleashed_mode: std::env::var("UNLEASHED_MODE").map(|v| v.to_lowercase() == "true").unwrap_or(false),
                artifact_extension: ".mp4".to_string(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_config_load_defaults() {
        let config = FactoryConfig::default();
        assert_eq!(config.ollama_url, "http://localhost:11434/v1");
        assert_eq!(config.model_name, "qwen2.5-coder:32b");
    }

    #[test]
    fn test_config_load_from_file() {
        // 一時的な config.toml を作成 (toml 拡張子を付加してフォーマットを認識させる)
        let mut file = tempfile::Builder::new()
            .suffix(".toml")
            .tempfile()
            .unwrap();
        writeln!(file, "ollama_url = \"http://custom:11434/v1\"").unwrap();
        writeln!(file, "comfyui_api_url = \"ws://custom:8188/ws\"").unwrap();
        writeln!(file, "batch_size = 5").unwrap();
        writeln!(file, "comfyui_timeout_secs = 60").unwrap();
        writeln!(file, "model_name = \"custom-model\"").unwrap();
        writeln!(file, "comfyui_base_dir = \"custom_dir\"").unwrap();
        writeln!(file, "brave_api_key = \"\"").unwrap();
        writeln!(file, "export_dir = \"/tmp/exports\"").unwrap();
        writeln!(file, "workspace_dir = \"./workspace\"").unwrap();
        writeln!(file, "clean_after_hours = 24").unwrap();
        writeln!(file, "sns_api_key = \"\"").unwrap();
        writeln!(file, "gemini_api_key = \"\"").unwrap();
        writeln!(file, "tiktok_api_key = \"\"").unwrap();
        writeln!(file, "script_model = \"gemini-2.0-flash\"").unwrap();
        writeln!(file, "unleashed_mode = false").unwrap();
        writeln!(file, "artifact_extension = \".mp4\"").unwrap();
        
        // config::File::from(path) を使って明示的なファイルを読み込む
        // 拡張子があるためフォーマットは自動判別される
        let settings = config::Config::builder()
            .add_source(config::File::from(file.path()))
            .build()
            .unwrap();
        
        let config: FactoryConfig = settings.try_deserialize().unwrap();
        assert_eq!(config.ollama_url, "http://custom:11434/v1");
        assert_eq!(config.model_name, "custom-model");
    }
}
