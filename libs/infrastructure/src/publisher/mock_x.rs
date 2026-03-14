/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

use aiome_core::error::AiomeError;
use aiome_core::traits::Publisher;
use async_trait::async_trait;
use std::path::PathBuf;
use tracing::info;

/// X (Twitter) Publisher (Mock version)
/// 今後の本番実装時には、Abyss Vault / KeyProxy を通じて実際の API (reqwest) を叩くように差し替えること。
pub struct MockXPublisher;

#[async_trait]
impl Publisher for MockXPublisher {
    async fn publish(
        &self,
        content: &str,
        media_paths: &[PathBuf],
        _metadata: &serde_json::Value,
    ) -> Result<String, AiomeError> {
        info!("𝕏 [MockX] Simulated Tweet: '{}'", content);
        if !media_paths.is_empty() {
            info!(
                "𝕏 [MockX] With {} media attachments: {:?}",
                media_paths.len(),
                media_paths
            );
        }

        // Mock Content ID
        let content_id = format!("x-{}", uuid::Uuid::new_v4());
        info!("𝕏 [MockX] Tweet Published! ContentID: {}", content_id);

        Ok(content_id)
    }

    fn platform_name(&self) -> &str {
        "X"
    }
}
