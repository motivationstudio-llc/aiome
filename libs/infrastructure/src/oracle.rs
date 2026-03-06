/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

use aiome_core::contracts::OracleVerdict;
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::LlmProvider;
use tracing::info;
use std::sync::Arc;

/// The Oracle (神託)
pub struct Oracle {
    provider: Arc<dyn LlmProvider>,
    soul_md: String,
}

impl Oracle {
    pub fn new(provider: Arc<dyn LlmProvider>, soul_md: String) -> Self {
        Self { 
            provider,
            soul_md 
        }
    }

    /// コンテンツの反響を評価し、最終審判（Verdict）を下す。
    pub async fn evaluate(
        &self,
        milestone_days: i64,
        topic: &str,
        style: &str,
        views: i64,
        likes: i64,
        comments_json: &str,
    ) -> Result<OracleVerdict, AiomeError> {
        info!("🔮 [Oracle] Evaluating Job ({}d): topic='{}', style='{}' using {}", milestone_days, topic, style, self.provider.name());

        let engagement_rate = if views > 0 { (likes as f64 / views as f64) * 100.0 } else { 0.0 };
        
        let preamble = format!(
            "AI の健全性を審判せよ。必ず JSON 形式で回答せよ。\n\n魂の美学:\n{}\n\nトピック: {}\nスタイル: {}\nViews: {}\nLikes: {}\nEngagement: {:.2}%\nコメント: {}",
            self.soul_md, topic, style, views, likes, engagement_rate, comments_json
        );

        let prompt_text = "審判を下せ。 JSON format: { \"alignment_score\": 0.0-1.0, \"growth_score\": 0.0-1.0, \"lesson\": \"string\", \"should_evolve\": bool, \"reasoning\": \"string\" }";

        let response = self.provider.complete(prompt_text, Some(&preamble)).await?;

        let json_str = crate::concept_manager::extract_json(&response)?;
        let verdict = serde_json::from_str::<OracleVerdict>(json_str.as_str())
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to parse Oracle JSON: {}", e) })?;

        info!("🔮 [Oracle] Verdict: Alignment={}, Growth={}, Evolve={}", 
            verdict.alignment_score, verdict.growth_score, verdict.should_evolve);

        Ok(verdict)
    }
}
