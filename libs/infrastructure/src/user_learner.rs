/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use aiome_core::llm_provider::LlmProvider;
use std::fs;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{info, warn};

pub struct UserLearner {
    provider: Arc<dyn LlmProvider + Send + Sync>,
    semaphore: Arc<Semaphore>,
}

impl UserLearner {
    pub fn new(provider: Arc<dyn LlmProvider + Send + Sync>, semaphore: Arc<Semaphore>) -> Self {
        Self {
            provider,
            semaphore,
        }
    }

    pub async fn learn_from_session(
        &self,
        conversation_summary: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let filename = "USER.md";
        let (user_path, current_user) = if let Ok(c) = fs::read_to_string(filename) {
            (filename.to_string(), c)
        } else if let Ok(c) = fs::read_to_string(format!("../../{}", filename)) {
            (format!("../../{}", filename), c)
        } else {
            (filename.to_string(), String::new())
        };

        if let Ok(_permit) = self.semaphore.try_acquire() {
            info!("🎓 [UserLearner] Analyzing session for user preference updates...");
            let prompt = format!(
                "この会話からユーザーの好みや情報を抽出し、USER.mdを更新してください。既存の情報は消さずに補完してください。\n\n現在のUSER.md:\n{}\n\n最近の会話内容:\n{}\n\nルール:\n1. 更新が必要なら、新しいUSER.mdの内容全体を出力せよ。\n2. 更新が不要なら「NO_UPDATE」とだけ出力せよ。\n3. フォーマットはMarkdownを維持せよ。日本語で出力せよ。",
                current_user, conversation_summary
            );

            match self.provider.complete(&prompt, None).await {
                Ok(reply) => {
                    let reply = reply.trim();
                    if reply != "NO_UPDATE" && !reply.is_empty() {
                        fs::write(&user_path, reply)?;
                        info!(
                            "✅ [UserLearner] {} has been updated based on session intelligence.",
                            user_path
                        );
                        return Ok(true);
                    }
                    info!("🎓 [UserLearner] No updates needed for {}.", user_path);
                }
                Err(e) => {
                    warn!("⚠️ [UserLearner] Failed to learn user preferences: {:?}", e);
                }
            }
        }
        Ok(false)
    }
}
