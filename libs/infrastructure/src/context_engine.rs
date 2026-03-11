/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use std::sync::Arc;
use aiome_core::llm_provider::LlmProvider;
use aiome_core::error::AiomeError;
use tokio::sync::Semaphore;
use tracing::{info, warn};
use crate::job_queue::SqliteJobQueue;

pub struct ContextEngine {
    provider: Arc<dyn LlmProvider + Send + Sync>,
    job_queue: Arc<SqliteJobQueue>,
    semaphore: Arc<Semaphore>,
}

impl ContextEngine {
    pub fn new(
        provider: Arc<dyn LlmProvider + Send + Sync>,
        job_queue: Arc<SqliteJobQueue>,
        semaphore: Arc<Semaphore>,
    ) -> Self {
        Self { provider, job_queue, semaphore }
    }

    /// Fetches the intelligent context for a channel (Summary + Recent turns)
    pub async fn get_intelligent_history(
        &self,
        channel_id: &str,
        max_recent_turns: i64,
    ) -> Result<(Option<String>, Vec<serde_json::Value>), AiomeError> {
        let summary = self.job_queue.get_chat_memory_summary(channel_id).await?;
        let history = self.job_queue.fetch_chat_history(channel_id, max_recent_turns).await?;
        Ok((summary, history))
    }

    /// Compresses history if it exceeds the threshold
    pub async fn maintain_context(&self, channel_id: &str, threshold: usize) -> Result<(), AiomeError> {
        // Fetch more than recent to check for compression need
        let all_recent = self.job_queue.fetch_chat_history(channel_id, (threshold * 2) as i64).await?;
        
        if all_recent.len() > threshold {
            if let Ok(_permit) = self.semaphore.try_acquire() {
                info!("🧠 [ContextEngine] Compressing history for channel: {}", channel_id);
                
                let current_summary = self.job_queue.get_chat_memory_summary(channel_id).await?
                    .unwrap_or_else(|| "なし".to_string());
                
                // Take the oldest half to compress
                let to_compress = &all_recent[..all_recent.len() - (threshold / 2)];
                let recent_context = to_compress.iter()
                    .map(|m| format!("{}: {}", m["role"], m["content"]))
                    .collect::<Vec<_>>()
                    .join("\n");

                let prompt = format!(
                    "以下のこれまでの要約と新しい会話履歴の内容を統合し、簡潔かつ重要なコンテキストを保持した新しい要約を作成してください。\n\n現在の要約:\n{}\n\n追加の会話履歴:\n{}\n\n出力形式: 重要な事実、ユーザーの意図、現在の状況をまとめた日本語の段落。余計な挨拶は不要。",
                    current_summary, recent_context
                );

                match self.provider.complete(&prompt, None).await {
                    Ok(new_summary) => {
                        self.job_queue.update_chat_memory_summary(channel_id, new_summary.trim()).await?;
                        
                        // Mark compressed messages as distilled
                        if let Some(last_compressed) = to_compress.last() {
                            if let Some(last_id) = last_compressed["id"].as_i64() {
                                let _ = self.job_queue.mark_chats_as_distilled(channel_id, last_id).await;
                            }
                        }
                        
                        info!("✅ [ContextEngine] Context compressed successfully for {}", channel_id);
                    }
                    Err(e) => {
                        warn!("⚠️ [ContextEngine] Failed to compress context: {:?}", e);
                    }
                }
            }
        }
        
        Ok(())
    }
}
