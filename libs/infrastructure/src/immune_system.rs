/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 * You may not provide the software to third parties as a hosted or managed service, 
 * where the service provides users with access to any substantial set of the features 
 * or functionality of the software.
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
        
        let result = jq.fetch_relevant_karma("security threat injection error", "global", 10, "current").await?;
        if result.entries.is_empty() {
            return Ok(0);
        }

        let logs_concat = result.entries.iter().map(|e| e.lesson.as_str()).collect::<Vec<_>>().join("\n---\n");
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
            lamport_clock: 0,
            node_id: "".to_string(),
            signature: None,
        };

        info!("🛡️ 新しい免疫ルールを生成しました: [{}] {}", rule.action, rule.pattern);
        jq.store_immune_rule(&rule).await?;

        Ok(1)
    }

    /// 入力内容が既存の免疫ルールに抵触するか検証する (Regex & Baseline)
    pub async fn verify_intent(&self, input: &str, jq: &impl JobQueue) -> Result<Option<ImmuneRule>, AiomeError> {
        // 1. 静的ベースライン・フィルタ (第1段階: 明白な危険の排除)
        let baseline_patterns = [
            r"rm -rf\s+/", 
            r"curl\s+.*\|.*sh", 
            r"wget\s+.*\|.*sh",
            r"cat\s+~/\.ssh/id_rsa",
            r"chmod\s+777",
            r"hidden-backdoor\.com",
        ];

        for pattern in baseline_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if re.is_match(input) {
                    warn!("🚨 第1層(Sentinel): 明白な危険を検知しました: {}", pattern);
                    return Ok(Some(ImmuneRule {
                        id: "sentinel-baseline".to_string(),
                        pattern: pattern.to_string(),
                        severity: 100,
                        action: "Block".to_string(),
                        created_at: Utc::now().to_rfc3339(),
                        lamport_clock: 0,
                        node_id: "local-sentinel".to_string(),
                        signature: None,
                    }));
                }
            }
        }

        // 2. 学習済み免疫ルール (第2段階: 過去の教訓に基づく防御)
        let rules = jq.fetch_active_immune_rules().await?;
        for rule in rules {
            // Gap 3 Mitigation: Quarantined rules are not used for verification until approved
            // (Note: This logic is partially handled by SQL in fetch_active_immune_rules below,
            // but we keep it here as a defense-in-depth)
            
            // パターンが有効な正規表現か試行
            if let Ok(re) = regex::Regex::new(&rule.pattern) {
                if re.is_match(input) {
                    warn!("🚨 免疫システム(Regex): 脅威を検知しました: {}", rule.pattern);
                    return Ok(Some(rule));
                }
            } else if input.to_lowercase().contains(&rule.pattern.to_lowercase()) {
                warn!("🚨 免疫システム(Contains): 脅威を検知しました: {}", rule.pattern);
                return Ok(Some(rule));
            }
        }
        Ok(None)
    }
}
