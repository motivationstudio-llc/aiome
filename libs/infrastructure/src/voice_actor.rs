/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

use factory_core::contracts::{VoiceRequest, VoiceResponse};
use factory_core::traits::AgentAct;
use factory_core::error::FactoryError;
use async_trait::async_trait;
use tracing::{info, error};
use std::path::Path;
use std::time::Duration;

/// 音声合成アクター (Qwen3-TTS Client)
#[derive(Clone)]
pub struct VoiceActor {
    server_url: String,
    default_voice: String,
    client: reqwest::Client,
}

impl VoiceActor {
    pub fn new(server_url: &str, default_voice: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            server_url: server_url.trim_end_matches('/').to_string(),
            default_voice: default_voice.to_string(),
            client,
        }
    }

    /// テキスト浄化パイプライン
    fn sanitize_for_tts(text: &str) -> String {
        let mut t = String::with_capacity(text.len());

        // 1. 制御文字・絵文字の除去
        for c in text.chars() {
            if c.is_control() && c != '\n' {
                continue;
            }
            let cp = c as u32;
            if (0x1F600..=0x1F64F).contains(&cp)
                || (0x1F300..=0x1F5FF).contains(&cp)
                || (0x1F680..=0x1F6FF).contains(&cp)
                || (0x1F900..=0x1F9FF).contains(&cp)
                || (0x2600..=0x26FF).contains(&cp)
                || (0x2700..=0x27BF).contains(&cp)
                || (0xFE00..=0xFE0F).contains(&cp)
                || (0x200D..=0x200D).contains(&cp)
            {
                continue;
            }
            t.push(c);
        }

        // 2. 三点リーダーの除去
        t = t.replace("…", "、")
             .replace("...", "、")
             .replace("..", "、");

        // 3. 連続空白・句読点の正規化
        while t.contains("  ") { t = t.replace("  ", " "); }
        while t.contains("。。") { t = t.replace("。。", "。"); }
        while t.contains("、、") { t = t.replace("、、", "、"); }
        t = t.replace("、。", "。");

        t.trim().to_string()
    }

    /// 言語別のデフォルトスピード設定
    fn default_speed_for_lang(lang: &str) -> f32 {
        match lang {
            "ja" => 1.1, // 日本語は少し早めが聞きやすい
            "en" => 1.0, // 英語はQwen3の滑舌維持のため標準
            _ => 1.0,
        }
    }
}

#[async_trait]
impl AgentAct for VoiceActor {
    type Input = VoiceRequest;
    type Output = VoiceResponse;

    async fn execute(
        &self,
        input: Self::Input,
        jail: &bastion::fs_guard::Jail,
    ) -> Result<Self::Output, FactoryError> {
        let sanitized_text = Self::sanitize_for_tts(&input.text);
        if sanitized_text.is_empty() {
            return Err(FactoryError::TtsFailure {
                reason: "Sanitized text is empty.".into(),
            });
        }

        let lang = input.lang.as_deref().unwrap_or("ja");
        
        let voice = if !input.voice.is_empty() {
            input.voice.clone()
        } else {
            match lang {
                "en" => "aiome_en".to_string(), 
                "ja" => self.default_voice.clone(),
                _ => self.default_voice.clone(),
            }
        };

        let speed = input.speed.unwrap_or_else(|| Self::default_speed_for_lang(lang));
        let model_name = input.model_name.as_deref().unwrap_or(&voice);
        let style = input.style.as_deref().unwrap_or("Neutral");
        let length = 1.0 / speed;

        info!(
            "🗣️ VoiceActor: Synthesizing with Style-Bert-VITS2 [model: {}, style: {}, length: {:.2}] for: '{}'",
            model_name,
            style,
            length,
            sanitized_text.chars().take(80).collect::<String>()
        );

        let url = format!("{}/voice", self.server_url);
        
        let response = self.client.post(&url)
            .query(&[
                ("text", &sanitized_text),
                ("model_name", &model_name.to_string()),
                ("style", &style.to_string()),
                ("length", &length.to_string()), 
                ("save_path", &"".to_string()),
            ])
            .send()
            .await
            .map_err(|e| FactoryError::TtsFailure {
                reason: format!("Failed to connect to TTS: {}", e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let err_body = response.text().await.unwrap_or_default();
            error!("TTS Server Error [{}]: {}", status, err_body);
            return Err(FactoryError::TtsFailure {
                reason: format!("TTS Server Error [{}]: {}", status, err_body),
            });
        }

        let audio_bytes = response.bytes().await
            .map_err(|e| FactoryError::TtsFailure {
                reason: format!("Failed to read data: {}", e),
            })?;

        let output_filename = format!("voice_{}.wav", uuid::Uuid::new_v4());
        let output_relative = Path::new("assets/audio").join(&output_filename);
        jail.create_dir_all("assets/audio").map_err(|e| FactoryError::Infrastructure {
            reason: format!("Failed to create audio directory: {}", e),
        })?;
        let output_abs = jail.root().join(&output_relative);

        std::fs::write(&output_abs, &audio_bytes)
            .map_err(|e| FactoryError::Infrastructure {
                reason: format!("Failed to write audio: {}", e),
            })?;

        info!("✅ VoiceActor: Synthesis completed: {}", output_relative.display());
        Ok(VoiceResponse {
            audio_path: output_relative.to_str().unwrap_or_default().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_for_tts() {
        let t = VoiceActor::sanitize_for_tts("テスト🎉です😊");
        assert_eq!(t, "テストです");
    }

    #[test]
    fn test_sanitize_removes_ellipsis() {
        let t = VoiceActor::sanitize_for_tts("未来は…ここにある。");
        assert_eq!(t, "未来は、ここにある。");
    }

    #[test]
    fn test_sanitize_normalizes_punctuation() {
        let t = VoiceActor::sanitize_for_tts("テスト。。重複。");
        assert_eq!(t, "テスト。重複。");
    }
}
