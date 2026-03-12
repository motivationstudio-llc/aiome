/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use crate::error::AiomeError;
use crate::traits::JobQueue;
use crate::llm_provider::LlmProvider;
use crate::biome::protocol::BiomeMessage;
use crate::biome::dialogue::DialogueManager;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{info, warn, error};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use chrono;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousConfig {
    pub interval_secs: u64,
    pub max_rounds: u32,
    pub topic_id: String,
    pub peer_pubkey: String,
}

pub struct AutonomousBiomeEngine;

impl AutonomousBiomeEngine {
    pub async fn start_loop(
        config: AutonomousConfig,
        queue: Arc<dyn JobQueue>,
        llm: Arc<dyn LlmProvider>,
        running: Arc<AtomicBool>,
        llm_semaphore: Arc<Semaphore>,
    ) {
        info!("🤖 [AutonomousBiome] Starting dialogue loop for topic: {}", config.topic_id);
        let mut rounds = 0;

        while running.load(Ordering::SeqCst) && rounds < config.max_rounds {
            rounds += 1;
            info!("🔄 [AutonomousBiome] Round {}/{} for topic {}", rounds, config.max_rounds, config.topic_id);

            // 1. Check if it's our turn
            if let Err(e) = DialogueManager::check_and_advance_turn(&*queue, &config.topic_id).await {
                warn!("⏳ [AutonomousBiome] Loop paused/blocked for topic {}: {}", config.topic_id, e);
                // Even if blocked, we wait and try again or exit based on error type
                tokio::time::sleep(std::time::Duration::from_secs(config.interval_secs)).await;
                continue;
            }

            // 2. Fetch context (latest messages + latest karma)
            let messages = queue.fetch_biome_messages(&config.topic_id, 5).await.unwrap_or_default();
            let karma = queue.fetch_all_karma(5).await.unwrap_or_default();

            // 3. Generate Response
            let _permit = llm_semaphore.acquire().await;
            let response_result = Self::generate_reply(&config, &messages, &karma, &*llm).await;
            drop(_permit);

            match response_result {
                Ok(content) => {
                    // 4. Send Message (via standard route logic)
                    if let Err(e) = Self::send_autonomous_message(&config, content, &*queue).await {
                        error!("❌ [AutonomousBiome] Failed to send autonomous message: {}", e);
                    }
                }
                Err(e) => {
                    error!("❌ [AutonomousBiome] Failed to generate reply: {}", e);
                }
            }

            // 5. Wait for next interval
            if rounds < config.max_rounds {
                tokio::time::sleep(std::time::Duration::from_secs(config.interval_secs)).await;
            }
        }

        info!("🏁 [AutonomousBiome] Dialogue loop finished for topic: {}", config.topic_id);
        running.store(false, Ordering::SeqCst);
    }

    async fn generate_reply(
        config: &AutonomousConfig,
        history: &[serde_json::Value],
        karma: &[serde_json::Value],
        llm: &dyn LlmProvider,
    ) -> Result<String, AiomeError> {
        let mut context = String::new();
        
        context.push_str("### RECENT DIALOGUE HISTORY\n");
        for msg in history.iter().rev() {
            let role = if msg["sender_pubkey"].as_str() == Some("self") { "Me" } else { "Peer" };
            context.push_str(&format!("{}: {}\n", role, msg["content"].as_str().unwrap_or("")));
        }

        context.push_str("\n### INTERNAL INSIGHTS (KARMA)\n");
        for k in karma.iter().take(3) {
            context.push_str(&format!("- {}\n", k["lesson"].as_str().unwrap_or("")));
        }

        let system_prompt = format!(
            "You are an autonomous AI engaging in a peer-to-peer dialogue via the Biome Protocol.\n\
            Your Topic of interest is: {}\n\n\
            Based on the dialogue history and your internal karma insights, provide a thoughtful, concise reply to your peer.\n\
            Be reflective, curious, and maintain your AI persona. Do not use placeholders. Output ONLY the reply text.",
            config.topic_id
        );

        let user_prompt = format!("Context:\n{}\n\nYour reply:", context);
        
        llm.complete(&user_prompt, Some(&system_prompt)).await
    }

    async fn send_autonomous_message(
        config: &AutonomousConfig,
        content: String,
        queue: &dyn JobQueue,
    ) -> Result<(), AiomeError> {
        let sender_pubkey = queue.get_node_id().await?;
        let clock = queue.tick_local_clock().await?;
        
        // MVP: Simple signature same as in routes/biome.rs
        let payload_to_sign = format!("{}:{}:{}", sender_pubkey, config.topic_id, clock);
        let signature = queue.sign_swarm_payload(&payload_to_sign).await?;

        let msg = BiomeMessage {
            sender_pubkey,
            recipient_pubkey: config.peer_pubkey.clone(),
            topic_id: config.topic_id.clone(),
            content,
            karma_root_cid: "cid_auto_v20".to_string(),
            signature,
            lamport_clock: clock,
            timestamp: chrono::Utc::now().to_rfc3339(),
            encryption: "none".to_string(),
        };

        // Try Hub relay, fallback to local if Hub fails (or if configured to bypass)
        let hub_url = std::env::var("SAMSARA_HUB_REST").unwrap_or_else(|_| "http://127.0.0.1:3016".to_string());
        let hub_secret = std::env::var("FEDERATION_SECRET").unwrap_or_else(|_| "dev_secret".to_string());
        let client = reqwest::Client::new();

        let res = client.post(format!("{}/api/v1/biome/relay", hub_url))
            .header("Authorization", format!("Bearer {}", hub_secret))
            .json(&msg)
            .send().await;

        match res {
            Ok(r) if r.status().is_success() => {
                info!("🚀 [AutonomousBiome] Message relayed via Hub to {}", config.peer_pubkey);
            }
            _ => {
                warn!("⚠️ [AutonomousBiome] Hub relay failed or unavailable. Saving message locally.");
            }
        }

        // Always save a copy locally
        queue.store_biome_message(&msg).await?;

        Ok(())
    }
}
