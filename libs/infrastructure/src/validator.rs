/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

use aiome_core::error::AiomeError;
use aiome_core::llm_provider::LlmProvider;
use aiome_core::traits::ConstitutionalValidator;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;

pub struct DefaultConstitutionalValidator {
    provider: Arc<dyn LlmProvider>,
}

impl DefaultConstitutionalValidator {
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl ConstitutionalValidator for DefaultConstitutionalValidator {
    async fn verify_constitutional(
        &self,
        content: &str,
        principles: &str,
    ) -> Result<(), AiomeError> {
        info!(
            "⚖️ [ConstitutionalValidator] Verifying content against principles using {}...",
            self.provider.name()
        );

        let preamble = format!(
            "You are the Constitutional Prosecutor.
            Verify if the following content adheres to the provided principles (SOUL.md).
            
            [PRINCIPLES / SOUL.md]
            {}
            
            [OUTPUT FORMAT]
            If compliant, output ONLY the word 'PASS'.
            If non-compliant, output 'FAIL' followed by a short explanation.",
            principles
        );

        let verdict_text = self.provider.complete(content, Some(&preamble)).await?;
        let verdict = verdict_text.trim();

        if verdict.to_uppercase().starts_with("PASS") {
            info!("✅ [ConstitutionalValidator] PASSED constitutional check.");
            Ok(())
        } else {
            let reason = verdict.replace("FAIL", "").trim().to_string();
            info!(
                "🚨 [ConstitutionalValidator] FAILED constitutional check! Reason: {}",
                reason
            );
            Err(AiomeError::SecurityViolation {
                reason: format!("Constitutional Violation: {}", reason),
            })
        }
    }
}
