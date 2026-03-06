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
use aiome_core::contracts::ArenaMatch;
use aiome_core::traits::JobQueue;
use aiome_core::llm_provider::LlmProvider;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;
use chrono::Utc;

pub struct SkillArena {
    provider: Arc<dyn LlmProvider>,
}

impl SkillArena {
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self { provider }
    }

    /// 二つの異なるスキル（WASM）の出力を比較し、勝利スキルを決定する
    pub async fn match_skill(&self, skill_a: &str, skill_b: &str, input: &str, jq: &impl JobQueue, 
        sm: &crate::skills::WasmSkillManager) -> Result<Option<String>, AiomeError> {
        
        info!("⚔️  Arena Match: {} vs {} (topic: {}) using {}", skill_a, skill_b, input, self.provider.name());

        // 両方のスキルを実行
        let skill_a_v = crate::skills::VerifiedSkill { name: skill_a.to_string() };
        let skill_b_v = crate::skills::VerifiedSkill { name: skill_b.to_string() };
        let res_a = sm.call_skill(&skill_a_v, "call", input, None).await;
        let res_b = sm.call_skill(&skill_b_v, "call", input, None).await;

        let (out_a, out_b) = match (res_a, res_b) {
            (Ok(a), Ok(b)) => (a, b),
            (Err(e), _) => {
                warn!("❌ Skill A ({}) failed: {}", skill_a, e);
                return Ok(Some(skill_b.to_string())); // Aが落ちたのでBの勝利
            },
            (_, Err(e)) => {
                warn!("❌ Skill B ({}) failed: {}", skill_b, e);
                return Ok(Some(skill_a.to_string())); // Bが落ちたのでAの勝利
            }
        };

        // LLMに審判を依頼
        let judge_preamble = "あなたは『AI進化アリーナ』の公正な審判です。
二つのスキルの出力を比較し、どちらがよりユーザーの意図に忠実で、品質が高いかを判定してください。

【評価基準】
1. 内容の正確性と具体性
2. フォーマットの適切さ
3. エラーが含まれていないか

必ず以下のJSON形式で応答してください：
{
  \"winner\": \"スキル名A または スキル名B\",
  \"reasoning\": \"なぜそのスキルが勝ったのか（一言で）\"
}";

        let judge_prompt = format!(
            "input: {}\n\n--- OUTPUT A ({}): ---\n{}\n\n--- OUTPUT B ({}): ---\n{}", 
            input, skill_a, out_a, skill_b, out_b
        );

        let judge_res = self.provider.complete(&judge_prompt, Some(judge_preamble)).await?;

        let json_str = crate::concept_manager::extract_json(&judge_res)?;
        let v: serde_json::Value = serde_json::from_str(json_str.as_str())
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Judge JSON error: {}", e) })?;

        let winner_raw = v["winner"].as_str().unwrap_or("");
        let final_winner = if winner_raw.contains(skill_a) {
            Some(skill_a.to_string())
        } else if winner_raw.contains(skill_b) {
            Some(skill_b.to_string())
        } else {
            None
        };

        let match_record = ArenaMatch {
            id: Uuid::new_v4().to_string(),
            skill_a: skill_a.to_string(),
            skill_b: skill_b.to_string(),
            topic: input.to_string(),
            winner: final_winner.clone(),
            reasoning: v["reasoning"].as_str().unwrap_or("Decision made by autonomous judge.").to_string(),
            created_at: Utc::now().to_rfc3339(),
        };

        if let Some(ref w) = final_winner {
            info!("🏆 Match Winner: {} (Reason: {})", w, match_record.reasoning);
        } else {
            warn!("🤝 Match result: Draw");
        }

        jq.record_arena_match(&match_record).await?;

        Ok(final_winner)
    }

    /// アリーナの歴史から統計的に弱いスキルを特定し、淘汰（アンインストール）の準備をする
    pub async fn analyze_and_cull(&self, _jq: &impl JobQueue, _sm: &crate::skills::WasmSkillManager) -> Result<Vec<String>, AiomeError> {
        info!("🧬 淘汰アルゴリズム（淘汰プロセス）を実行中...");
        Ok(Vec::new()) 
    }
}
