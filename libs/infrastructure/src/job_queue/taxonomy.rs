/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Elastic License 2.0 (ELv2).
 */

use aiome_core::contracts::KarmaClassification;
use aiome_core::error::AiomeError;
use aiome_core::llm_provider::LlmProvider;

/// Sprint 3-B: Hierarchical Classification (Taxonomy)
/// 過去の教訓をドメインとサブトピックに自動分類する。
pub struct KarmaTaxonomy;

impl KarmaTaxonomy {
    /// 指定された教訓（lesson）をLLMを用いて分類する。
    pub async fn classify(
        provider: &dyn LlmProvider,
        lesson: &str,
    ) -> Result<KarmaClassification, AiomeError> {
        let system_prompt = r#"You are the Karma Classifier for Aiome OS.
Your task is to classify a "lesson" (karma) into a hierarchical taxonomy.

Output MUST be a strict JSON object matching this structure:
{
  "domain": "Technical | Creative | Governance | Social | Meta",
  "subtopic": "string",
  "reasoning": "string"
}

Domains:
- Technical: Code, Performance, Bugs, API, Infrastructure.
- Creative: Aesthetics, Style, Tone, Visuals.
- Governance: Security, Policy, Ethics, Compliance.
- Social: User interaction, Engagement, Empathy.
- Meta: System evolution, Learning patterns, Self-improvement.

Constraint: Output ONLY raw JSON. No markdown blocks."#;

        let prompt = format!("Lesson: \"{}\"", lesson);

        match provider.complete(&prompt, Some(system_prompt)).await {
            Ok(response) => {
                // R1 Defense: AI might still output markdown blocks
                let clean_json = response
                    .trim()
                    .trim_start_matches("```json")
                    .trim_start_matches("```")
                    .trim_end_matches("```")
                    .trim();

                serde_json::from_str::<KarmaClassification>(clean_json).map_err(|e| {
                    tracing::warn!("🧬 [Taxonomy] JSON Parse Error: {}. Raw: {}", e, response);
                    AiomeError::Infrastructure {
                        reason: format!("Invalid classification format: {}", e),
                    }
                })
            }
            Err(e) => Err(e),
        }
    }

    /// フォールバック値を生成する（LLMエラー時）
    pub fn fallback() -> KarmaClassification {
        KarmaClassification {
            domain: "general".to_string(),
            subtopic: "uncategorized".to_string(),
            reasoning: "LLM classification failed, using fallback.".to_string(),
        }
    }
}
