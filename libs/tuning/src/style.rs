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
use std::collections::HashMap;
use std::path::Path;
use factory_core::error::FactoryError;

/// 演出プロファイル（スタイル）の定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleProfile {
    /// プロファイル名
    pub name: String,
    /// 説明
    pub description: String,
    
    // --- 視覚演出 (Cameraman) ---
    /// ズーム速度 (0.001 - 0.005)
    pub zoom_speed: f64,
    /// パンの強さ (0.0 - 1.0)
    pub pan_intensity: f64,
    
    // --- 音響演出 (SoundMixer) ---
    /// BGM音量 (0.0 - 1.0)
    pub bgm_volume: f32,
    /// ダッキング閾値 (dB, 例: -20.0)
    pub ducking_threshold: f32,
    /// ダッキング時のBGM倍率 (0.0 - 1.0, 0.4等)
    pub ducking_ratio: f32,
    /// フェードアウト時間 (秒)
    pub fade_duration: f32,
}

impl Default for StyleProfile {
    fn default() -> Self {
        Self {
            name: "default".into(),
            description: "標準的な演出設定".into(),
            zoom_speed: 0.0015,
            pan_intensity: 0.5,
            bgm_volume: 0.15,
            ducking_threshold: 0.1, // sidechaincompress の threshold
            ducking_ratio: 0.4,
            fade_duration: 3.0,
        }
    }
}

/// 演出スタイルを管理するマネージャ
pub struct StyleManager {
    profiles: HashMap<String, StyleProfile>,
}

impl StyleManager {
    /// styles.toml からプロファイルをロードする
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, FactoryError> {
        let content = std::fs::read_to_string(path).map_err(|e| FactoryError::ConfigLoad {
            source: anyhow::anyhow!("Failed to read styles.toml: {}", e),
        })?;
        
        let config: HashMap<String, StyleProfile> = toml::from_str(&content).map_err(|e| FactoryError::ConfigLoad {
            source: anyhow::anyhow!("Failed to parse styles.toml: {}", e),
        })?;
        
        Ok(Self { profiles: config })
    }

    /// デフォルト設定のみのマネージャを作成
    pub fn new_empty() -> Self {
        let mut profiles = HashMap::new();
        profiles.insert("default".into(), StyleProfile::default());
        Self { profiles }
    }

    /// 特定のスタイルを取得（存在しない場合は default）
    pub fn get_style(&self, name: &str) -> StyleProfile {
        self.profiles.get(name).cloned().unwrap_or_else(|| {
            tracing::warn!("Style '{}' not found, falling back to default", name);
            self.profiles.get("default").cloned().unwrap_or_default()
        })
    }

    /// 利用可能なスタイル名の一覧を取得（LLM提示用）
    pub fn list_available_styles(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.profiles.keys().cloned().collect();
        keys.sort();
        keys
    }

    /// プロファイルの説明を含めた詳細な一覧を取得（LLM提示用）
    pub fn get_style_descriptions(&self) -> String {
        let mut desc = String::new();
        for profile in self.profiles.values() {
            desc.push_str(&format!("- {}: {}
", profile.name, profile.description));
        }
        desc
    }
}
