/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

use factory_core::contracts::OracleVerdict;
use factory_core::error::FactoryError;
use rig::providers::gemini;
use rig::prelude::*;
use rig::completion::Prompt;
use tracing::info;
use secrecy::{SecretString, ExposeSecret};

/// The Oracle (神託)
pub struct Oracle {
    api_key: SecretString,
    model_name: String,
    soul_md: String,
}

impl Oracle {
    pub fn new(api_key: &str, model_name: &str, soul_md: String) -> Self {
        Self { 
            api_key: SecretString::new(api_key.into()), 
            model_name: model_name.to_string(), 
            soul_md 
        }
    }

    /// 動画の反響を評価し、最終審判（Verdict）を下す。
    pub async fn evaluate(
        &self,
        milestone_days: i64,
        topic: &str,
        style: &str,
        views: i64,
        likes: i64,
        comments_json: &str,
    ) -> Result<OracleVerdict, FactoryError> {
        info!("🔮 [Oracle] Evaluating Job ({}d): topic='{}', style='{}'", milestone_days, topic, style);

        let engagement_rate = if views > 0 { (likes as f64 / views as f64) * 100.0 } else { 0.0 };
        
        let client = gemini::Client::new(self.api_key.expose_secret())
            .map_err(|e| FactoryError::Infrastructure { reason: e.to_string() })?;

        let preamble = format!(
            "AI の健全性を審判せよ。必ず JSON 形式で回答せよ。\n\n魂の美学:\n{}\n\nトピック: {}\nスタイル: {}\nViews: {}\nLikes: {}\nEngagement: {:.2}%\nコメント: {}",
            self.soul_md, topic, style, views, likes, engagement_rate, comments_json
        );

        let agent = client.agent(&self.model_name).preamble(&preamble).build();
        let prompt_text = "審判を下せ。 JSON format: { \"alignment_score\": 0.0-1.0, \"growth_score\": 0.0-1.0, \"lesson\": \"string\", \"should_evolve\": bool, \"reasoning\": \"string\" }";

        let response: String = agent.prompt(prompt_text).await
            .map_err(|e| FactoryError::Infrastructure { reason: e.to_string() })?;

        let json_str = crate::concept_manager::extract_json(&response)?;
        let verdict = serde_json::from_str::<OracleVerdict>(json_str.as_str())
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Failed to parse Oracle JSON: {}", e) })?;

        info!("🔮 [Oracle] Verdict: Alignment={}, Growth={}, Evolve={}", 
            verdict.alignment_score, verdict.growth_score, verdict.should_evolve);

        Ok(verdict)
    }
}
