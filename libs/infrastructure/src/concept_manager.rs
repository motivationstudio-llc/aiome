/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

use aiome_core::contracts::{ConceptRequest, ConceptResponse, LocalizedScript};
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::LlmProvider;
use aiome_core::traits::AgentAct;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;

/// コンセプト生成機 (Director)
pub struct ConceptManager {
    main_provider: Arc<dyn LlmProvider>,
    prosecutor_provider: Option<Arc<dyn LlmProvider>>,
    soul_md: Option<String>,
}

impl ConceptManager {
    pub fn new(main_provider: Arc<dyn LlmProvider>) -> Self {
        Self {
            main_provider,
            prosecutor_provider: None,
            soul_md: None,
        }
    }

    pub fn with_constitutional_layer(
        mut self,
        prosecutor: Arc<dyn LlmProvider>,
        soul_md: &str,
    ) -> Self {
        self.prosecutor_provider = Some(prosecutor);
        self.soul_md = Some(soul_md.to_string());
        self
    }

    /// 検察官 (Constitutional Prosecutor) による出力検証
    async fn verify_with_prosecutor(&self, concept: &ConceptResponse) -> Result<(), AiomeError> {
        if let Some(p) = &self.prosecutor_provider {
            use aiome_core::traits::ConstitutionalValidator;
            let validator = crate::validator::DefaultConstitutionalValidator::new(p.clone());
            let concept_summary = format!(
                "Title: {}\nIntro: {}\nBody: {}\nOutro: {}\nVisuals: {:?}",
                concept.title,
                concept.display_intro,
                concept.display_body,
                concept.display_outro,
                concept.visual_prompts
            );
            validator
                .verify_constitutional(&concept_summary, self.soul_md.as_deref().unwrap_or(""))
                .await?;
        }
        Ok(())
    }
}

#[async_trait]
impl AgentAct for ConceptManager {
    type Input = ConceptRequest;
    type Output = ConceptResponse;

    async fn execute(
        &self,
        input: Self::Input,
        _jail: &bastion::fs_guard::Jail,
    ) -> Result<Self::Output, AiomeError> {
        info!(
            "🎬 ConceptManager: Starting 2-stage concept generation for topic '{}'...",
            input.topic
        );

        let concept = self.generate_english_concept(&input).await?;
        self.verify_with_prosecutor(&concept).await?;

        let mut concept = concept;
        let ja_script = self.translate_to_japanese(&concept).await?;

        concept.scripts = vec![
            LocalizedScript {
                lang: "en".to_string(),
                display_intro: concept.display_intro.clone(),
                display_body: concept.display_body.clone(),
                display_outro: concept.display_outro.clone(),
                script_intro: concept.script_intro.clone(),
                script_body: concept.script_body.clone(),
                script_outro: concept.script_outro.clone(),
                style_intro: concept.style_intro.clone(),
                style_body: concept.style_body.clone(),
                style_outro: concept.style_outro.clone(),
            },
            ja_script.clone(),
        ];

        concept.display_intro = ja_script.display_intro;
        concept.display_body = ja_script.display_body;
        concept.display_outro = ja_script.display_outro;
        concept.script_intro = ja_script.script_intro;
        concept.script_body = ja_script.script_body;
        concept.script_outro = ja_script.script_outro;

        Ok(concept)
    }
}

impl ConceptManager {
    async fn generate_english_concept(
        &self,
        input: &ConceptRequest,
    ) -> Result<ConceptResponse, AiomeError> {
        let preamble = "You are a professional content producer. Generate content in English. Return ONLY JSON.";

        let prompt_text = format!(
            "Topic: {}\nTrend: {:?}\nKarma: {:?}",
            input.topic, input.trend_items, input.relevant_karma
        );

        let response = self
            .main_provider
            .complete(&prompt_text, Some(preamble))
            .await?;
        let json_str = extract_json(&response)?;

        let concept: ConceptResponse =
            serde_json::from_str(&json_str).map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

        Ok(concept)
    }

    async fn translate_to_japanese(
        &self,
        concept: &ConceptResponse,
    ) -> Result<LocalizedScript, AiomeError> {
        let preamble = "Translate the content into natural Japanese. Return ONLY JSON.";
        let prompt_text = format!("EN Content: {:?}", concept);

        let response = self
            .main_provider
            .complete(&prompt_text, Some(preamble))
            .await?;
        let json_str = extract_json(&response)?;

        let mut script: LocalizedScript =
            serde_json::from_str(&json_str).map_err(|e| AiomeError::Infrastructure {
                reason: e.to_string(),
            })?;

        script.lang = "ja".to_string();
        Ok(script)
    }
}

use shared::output_validator;

pub fn extract_json(text: &str) -> Result<String, AiomeError> {
    let block = output_validator::extract_json_block(text);
    if block.trim().is_empty() || (!block.contains('{') && !block.contains('[')) {
        return Err(AiomeError::Infrastructure {
            reason: "No JSON block detected in LLM output".into(),
        });
    }
    Ok(block)
}
