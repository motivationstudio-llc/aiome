/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use crate::error::AiomeError;
use crate::traits::JobQueue;
use chrono::{DateTime, Utc};

/// Biome プロトコルの対話ルール
/// - State Channel: 1件のトピックにつき最大10往復
/// - 往復終了後に LLM による要約を行い、アーカイブ化
pub const MAX_DIALOGUE_TURNS: i32 = 10;

pub struct DialogueManager;

impl DialogueManager {
    /// メッセージ送信前に制約（ターン制限、クールダウン）をチェックし、問題なければターンを進める
    pub async fn check_and_advance_turn(
        queue: &dyn JobQueue,
        topic_id: &str,
    ) -> Result<(), AiomeError> {
        let status = queue.get_biome_topic_status(topic_id).await?;
        
        if let Some((turn_count, cooldown_until)) = status {
            // 1. ターン制限チェック
            if turn_count >= MAX_DIALOGUE_TURNS {
                return Err(AiomeError::Infrastructure {
                    reason: format!("Biome Error: Topic {} has reached MAX_DIALOGUE_TURNS ({})", topic_id, MAX_DIALOGUE_TURNS),
                });
            }

            // 2. クールダウンチェック
            if let Some(until_str) = cooldown_until {
                if let Ok(until) = DateTime::parse_from_rfc3339(&until_str) {
                    if Utc::now() < until.with_timezone(&Utc) {
                        return Err(AiomeError::Infrastructure {
                            reason: format!("Biome Error: Topic {} is in cooldown until {}", topic_id, until_str),
                        });
                    }
                }
            }
        }

        // 3. ターン更新 (デフォルト5分のクールダウン)
        queue.advance_biome_turn(topic_id, 5).await?;

        Ok(())
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
