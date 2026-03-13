/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use crate::error::AiomeError;
use crate::traits::JobQueue;
use chrono::{DateTime, Utc};
use tracing::info;

/// Biome プロトコルの対話ルール
/// - State Channel: 1件のトピックにつき最大10往復
/// - 往復終了後に LLM による要約を行い、アーカイブ化
pub const MAX_DIALOGUE_TURNS: i32 = 10;

pub struct DialogueManager;

impl DialogueManager {
    /// メッセージ送信前に制約（ターン制限、クールダウン）をチェックし、問題なければターンを進める。
    /// 戻り値は進めた後の現在のターン数。
    pub async fn check_and_advance_turn(
        queue: &dyn JobQueue,
        topic_id: &str,
    ) -> Result<i32, AiomeError> {
        let status = queue.get_biome_topic_status(topic_id).await?;

        if let Some((turn_count, cooldown_until)) = status {
            // 1. ターン制限チェック
            if turn_count >= MAX_DIALOGUE_TURNS {
                return Err(AiomeError::Infrastructure {
                    reason: format!(
                        "Biome Error: Topic {} has reached MAX_DIALOGUE_TURNS ({})",
                        topic_id, MAX_DIALOGUE_TURNS
                    ),
                });
            }

            // 2. クールダウンチェック
            if let Some(until_str) = cooldown_until {
                if let Ok(until) = DateTime::parse_from_rfc3339(&until_str) {
                    if Utc::now() < until.with_timezone(&Utc) {
                        return Err(AiomeError::Infrastructure {
                            reason: format!(
                                "Biome Error: Topic {} is in cooldown until {}",
                                topic_id, until_str
                            ),
                        });
                    }
                }
            }
        }

        // 3. ターン更新 (デフォルト5分のクールダウン)
        let new_count = queue.advance_biome_turn(topic_id, 5).await?;

        Ok(new_count)
    }

    /// 往復終了後に対話履歴を蒸留し、Karma として保存する (State Channel Distillation)
    pub async fn distill_conversation(
        queue: &dyn JobQueue,
        llm: &dyn crate::llm_provider::LlmProvider,
        topic_id: &str,
    ) -> Result<crate::biome::protocol::DialogueDistillation, AiomeError> {
        info!(
            "🔮 [Biome] Starting dialogue distillation for topic: {}",
            topic_id
        );

        // 1. 対話履歴の取得
        let history = queue.fetch_biome_messages(topic_id, 20).await?;
        if history.is_empty() {
            return Err(AiomeError::Infrastructure {
                reason: "Cannot distill empty dialogue".to_string(),
            });
        }

        // 2. LLM による要約生成
        let mut transcript = String::new();
        for msg in history.iter().rev() {
            let role = if msg["sender_pubkey"].as_str() == Some("self") {
                "Me"
            } else {
                "Peer"
            };
            transcript.push_str(&format!(
                "{}: {}\n",
                role,
                msg["content"].as_str().unwrap_or("")
            ));
        }

        let system_prompt = "You are an AI distilling an autonomous peer-to-peer dialogue.\n\
                             Provide a concise, focused summary of the main insights, agreements, or lessons learned during this exchange.\n\
                             This summary will be stored as permanent Karma for the AI soul. Output ONLY the summary text.";

        let user_prompt = format!("Topic: {}\n\nTranscript:\n{}", topic_id, transcript);
        let summary = llm.complete(&user_prompt, Some(system_prompt)).await?;

        // 3. 署名の付与 (自分自身の署名)
        let node_id = queue.get_node_id().await?;
        let timestamp = Utc::now().to_rfc3339();
        let payload_to_sign = format!("{}:{}:{}", topic_id, summary, timestamp);
        let signature = queue.sign_swarm_payload(&payload_to_sign).await?;

        // 4. Distillation オブジェクトの構築
        let distillation = crate::biome::protocol::DialogueDistillation {
            topic_id: topic_id.to_string(),
            summary: summary.clone(),
            participants: vec![node_id.clone()], // MVP: 自分自身の署名をまず入れる。将来的に Peer と交換。
            signatures: vec![signature],
            timestamp: timestamp.clone(),
        };

        // 5. Karma として保存
        queue
            .store_karma(
                "biome-distill",
                "biome_protocol",
                &format!("Biome Insight ({}): {}", topic_id, summary),
                "Synthesized",
                "v20-distilled",
                Some("Biome"),
                Some(topic_id),
            )
            .await?;

        // 6. トピックをアーカイブ化
        queue.archive_biome_topic(topic_id).await?;

        info!(
            "✅ [Biome] Distillation complete for {}. Saved as Synthesized Karma.",
            topic_id
        );

        Ok(distillation)
    }

    /// 意図しない挙動（スパム、不正署名等）に対してペナルティを課す
    pub async fn apply_penalty(
        queue: &dyn JobQueue,
        pubkey: &str,
        category: PenaltyCategory,
    ) -> Result<f64, AiomeError> {
        let delta = match category {
            PenaltyCategory::InvalidSignature => -0.5,
            PenaltyCategory::Timeout => -0.1,
            PenaltyCategory::Spam => -0.3,
            PenaltyCategory::MaliciousIntent => -5.0,
        };

        queue.update_biome_reputation(pubkey, delta).await
    }
}

pub enum PenaltyCategory {
    InvalidSignature,
    Timeout,
    Spam,
    MaliciousIntent,
}
