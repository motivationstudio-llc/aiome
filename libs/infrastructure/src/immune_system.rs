/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

use aiome_core::error::AiomeError;
use aiome_core::contracts::ImmuneRule;
use aiome_core::traits::JobQueue;
use aiome_core::llm_provider::LlmProvider;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;
use chrono::Utc;

pub struct AdaptiveImmuneSystem {
    provider: Arc<dyn LlmProvider>,
}

impl AdaptiveImmuneSystem {
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self { provider }
    }

    /// 失敗ログやセキュリティインシデントを分析し、新しい免疫ルールを生成する
    pub async fn analyze_threats(&self, jq: &impl JobQueue) -> Result<u32, AiomeError> {
        info!("防御システム: 脅威分析を開始中 (using {})...", self.provider.name());
        
        let recent_karma = jq.fetch_relevant_karma("security threat injection error", "global", 10, "current").await?;
        if recent_karma.is_empty() {
            return Ok(0);
        }

        let logs_concat = recent_karma.join("\n---\n");
        let preamble = "あなたはシステムの自己防衛エンジンです。以下のログから攻撃パターンを特定し、防御ルールを1つ JSON 形式で作成してください。\nFormat: {\"pattern\": \"攻撃的な単語や正規表現\", \"severity\": 0-100, \"action\": \"Block/Alert\"}";

        let response = self.provider.complete(&logs_concat, Some(preamble)).await?;

        let json_str = crate::concept_manager::extract_json(&response)?;
        let v: serde_json::Value = serde_json::from_str(json_str.as_str())
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Failed to parse immune rule JSON: {}", e) })?;

        let rule = ImmuneRule {
            id: Uuid::new_v4().to_string(),
            pattern: v["pattern"].as_str().unwrap_or("unknown").to_string(),
            severity: v["severity"].as_u64().unwrap_or(50) as u8,
            action: v["action"].as_str().unwrap_or("Block").to_string(),
            created_at: Utc::now().to_rfc3339(),
        };

        info!("🛡️ 新しい免疫ルールを生成しました: [{}] {}", rule.action, rule.pattern);
        jq.store_immune_rule(&rule).await?;

        Ok(1)
    }

    /// 入力内容が既存の免疫ルールに抵触するか検証する
    pub async fn verify_intent(&self, input: &str, jq: &impl JobQueue) -> Result<Option<ImmuneRule>, AiomeError> {
        let rules = jq.fetch_active_immune_rules().await?;
        for rule in rules {
            if input.to_lowercase().contains(&rule.pattern.to_lowercase()) {
                warn!("🚨 免疫システムが脅威を検知・遮断しました: {}", rule.pattern);
                return Ok(Some(rule));
            }
        }
        Ok(None)
    }
}
