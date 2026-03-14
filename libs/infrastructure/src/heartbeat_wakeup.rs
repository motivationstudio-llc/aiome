/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

use aiome_core::llm_provider::LlmProvider;
use std::fs;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{info, warn};

pub struct HeartbeatWakeupService {
    provider: Arc<dyn LlmProvider + Send + Sync>,
    semaphore: Arc<Semaphore>,
}

impl HeartbeatWakeupService {
    pub fn new(provider: Arc<dyn LlmProvider + Send + Sync>, semaphore: Arc<Semaphore>) -> Self {
        Self {
            provider,
            semaphore,
        }
    }

    pub async fn run_wakeup_ping(&self) -> Option<String> {
        let filename = "HEARTBEAT.md";
        let content = if let Ok(c) = fs::read_to_string(filename) {
            c
        } else if let Ok(c) = fs::read_to_string(format!("../../{}", filename)) {
            c
        } else {
            String::new()
        };

        if self.is_effectively_empty(&content) {
            return None;
        }

        // Phase 1 Flaw 4 Defense: Use try_acquire to avoid blocking background worker if busy
        if let Ok(_permit) = self.semaphore.try_acquire() {
            info!("💓 [Heartbeat] Triggering Wakeup Ping...");

            // Phase 1 Flaw 7 Defense: Strict instructions & Last Run context (simulated in prompt for now)
            let prompt = format!(
                "[System: Wakeup Ping]\n\
                 Read HEARTBEAT.md and check for pending tasks.\n\
                 もし現在やるべきことがなければ、絶対に何もしないで 'HEARTBEAT_OK' とだけ答えよ。\n\n\
                 HEARTBEAT.md:\n{}\n\n\
                 RULES:\n\
                 1. If nothing needs attention, reply exactly 'HEARTBEAT_OK'.\n\
                 2. If there are tasks (marked with [ ]), execute them or notify the user.\n\
                 3. Never repeat information unless specifically asked.",
                content
            );

            match self.provider.complete(&prompt, None).await {
                Ok(reply) => {
                    let reply = reply.trim();
                    if reply == "HEARTBEAT_OK" || reply.is_empty() {
                        info!("💓 [Heartbeat] System state: OK");
                        None
                    } else {
                        info!("💓 [Heartbeat] Proactive Talk generated.");
                        Some(reply.to_string())
                    }
                }
                Err(e) => {
                    warn!("⚠️ [Heartbeat] LLM completion failed: {:?}", e);
                    None
                }
            }
        } else {
            info!("💤 [Heartbeat] LLM busy, skipping wakeup ping.");
            None
        }
    }

    fn is_effectively_empty(&self, content: &str) -> bool {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // Skip markdown header lines (# followed by space or EOL, ## etc)
            if trimmed.starts_with('#') {
                let after_hash = &trimmed[1..];
                if after_hash.is_empty()
                    || after_hash
                        .chars()
                        .next()
                        .map(|c| c.is_whitespace())
                        .unwrap_or(false)
                {
                    continue;
                }
            }
            // Skip empty markdown list items like "- [ ]" or "* [ ]" or just "- "
            if (trimmed.starts_with("- [ ]")
                || trimmed.starts_with("* [ ]")
                || trimmed.starts_with("+ [ ]"))
                && trimmed.len() <= 5
            {
                continue;
            }
            if trimmed == "- " || trimmed == "* " || trimmed == "+ " {
                continue;
            }
            // Found actionable content
            return false;
        }
        true
    }
}
