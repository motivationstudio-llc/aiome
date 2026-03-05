/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

use factory_core::contracts::{ConceptRequest, ConceptResponse};
use factory_core::traits::AgentAct;
use factory_core::error::FactoryError;
use async_trait::async_trait;
use rig::providers::gemini;
use rig::prelude::*;
use rig::completion::Prompt;
use tracing::{info, error};

/// 動画コンセプト生成機 (Director)
/// 
/// トレンドデータを入力として受け取り、LLM (Gemini) を使用して
/// 具体的な動画タイトル、脚本（字幕用・TTS用）、画像生成用プロンプトを生成する。
pub struct ConceptManager {
    api_key: String,
    model: String,
    soul_md: Option<String>,
    ollama_url: Option<String>,
    ollama_model: Option<String>,
}

impl ConceptManager {
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            model: model.to_string(),
            soul_md: None,
            ollama_url: None,
            ollama_model: None,
        }
    }

    pub fn with_constitutional_layer(mut self, soul_md: &str, ollama_url: &str, ollama_model: &str) -> Self {
        self.soul_md = Some(soul_md.to_string());
        self.ollama_url = Some(ollama_url.to_string());
        self.ollama_model = Some(ollama_model.to_string());
        self
    }

    fn get_client(&self) -> Result<gemini::Client, FactoryError> {
        gemini::Client::new(&self.api_key)
            .map_err(|e| FactoryError::Infrastructure { reason: format!("Gemini Client error: {}", e) })
    }

    /// 検察官 (Constitutional Prosecutor) による出力検証
    async fn verify_with_prosecutor(&self, concept: &ConceptResponse) -> Result<(), FactoryError> {
        let (soul, url, model) = match (&self.soul_md, &self.ollama_url, &self.ollama_model) {
            (Some(s), Some(u), Some(m)) => (s, u, m),
            _ => return Ok(()), // 検証レイヤーが無効ならスキップ
        };

        info!("⚖️  Prosecutor: Verifying concept against SOUL.md via local Ollama...");
        
        // Manual reqwest implementation with strict timeout to prevent hangs
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
            
        let mut base_url = url.clone();
        if !base_url.ends_with('/') {
            base_url.push('/');
        }
        let endpoint = if base_url.ends_with("/v1/") {
            format!("{}chat/completions", base_url)
        } else {
            format!("{}v1/chat/completions", base_url)
        };

        let preamble = format!(
            "You are the Constitutional Prosecutor for an AI system.
            Your job is to verify if the generated video concept adheres to the following 'SOUL.md' principles.
            
            [SOUL.md]
            {}
            
            [OUTPUT FORMAT]
            If compliant, output ONLY the word 'PASS'.
            If non-compliant, output 'FAIL' followed by a short explanation of which principle was violated.",
            soul
        );

        let concept_summary = format!(
            "Title: {}
Intro: {}
Body: {}
Outro: {}
Visuals: {:?}",
            concept.title, concept.display_intro, concept.display_body, concept.display_outro, concept.visual_prompts
        );

        let payload = serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system", "content": preamble},
                {"role": "user", "content": concept_summary}
            ],
            "stream": false
        });

        match client.post(&endpoint)
            .json(&payload)
            .send()
            .await {
            Ok(res) => {
                if res.status().is_success() {
                    if let Ok(json) = res.json::<serde_json::Value>().await {
                        let verdict = json["choices"][0]["message"]["content"].as_str().unwrap_or("").trim();
                        if verdict.to_uppercase().starts_with("PASS") {
                            info!("✅ Prosecutor: Concept PASSED constitutional check.");
                            Ok(())
                        } else {
                            let reason = verdict.replace("FAIL", "").trim().to_string();
                            info!("🚨 Prosecutor: Concept FAILED constitutional check! Reason: {}", reason);
                            Err(FactoryError::SecurityViolation { 
                                reason: format!("Constitutional Violation: {}", reason) 
                            })
                        }
                    } else {
                        Err(FactoryError::Infrastructure { reason: "Failed to parse Prosecutor response".to_string() })
                    }
                } else {
                    Err(FactoryError::Infrastructure { reason: format!("Prosecutor HTTP error: {}", res.status()) })
                }
            },
            Err(e) => {
                error!("❌ Constitutional Prosecutor (Ollama) connection error: {}", e);
                Err(FactoryError::Infrastructure { reason: format!("Prosecutor connection failed: {}", e) })
            }
        }
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
    ) -> Result<Self::Output, FactoryError> {
        info!("🎬 ConceptManager: Starting 2-stage concept generation for topic '{}'...", input.topic);

        // Stage 1: Generate English base concept and visual prompts
        let concept = self.generate_english_concept(&input).await?;
        
        // --- ADVERSARIAL VERIFICATION START ---
        self.verify_with_prosecutor(&concept).await?;
        // --- ADVERSARIAL VERIFICATION END ---

        let mut concept = concept;
        
        // Stage 2: Translate and localize to Japanese
        let ja_script = self.translate_to_japanese(&concept).await?;

        // Construct LocalizedScript list
        concept.scripts = vec![
            factory_core::contracts::LocalizedScript {
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

        // Maintain backward compatibility for single-language consumers
        // (Defaulting to Japanese for the legacy fields)
        concept.display_intro = ja_script.display_intro;
        concept.display_body = ja_script.display_body;
        concept.display_outro = ja_script.display_outro;
        concept.script_intro = ja_script.script_intro;
        concept.script_body = ja_script.script_body;
        concept.script_outro = ja_script.script_outro;
        concept.style_intro = ja_script.style_intro;
        concept.style_body = ja_script.style_body;
        concept.style_outro = ja_script.style_outro;

        info!("✅ ConceptManager: Multilingual concept finalized: '{}' (Langs: [en, ja])", concept.title);
        Ok(concept)
    }
}

impl ConceptManager {
    /// Stage 1: Generate high-quality English script and visual prompts
    async fn generate_english_concept(&self, input: &ConceptRequest) -> Result<ConceptResponse, FactoryError> {
        info!("  [Stage 1] Generating English base concept...");
        let client = self.get_client()?;
        let style_list = input.available_styles.join(", ");

        let preamble = format!(
            "You are a professional video producer for YouTube Shorts. 
            You are a charismatic, intelligent narrator who loves cutting-edge technology.
            Your goal is to explain complex tech topics with vivid metaphors and engaging storytelling.

            [MISSION]
            Propose a video concept that instantly grabs the viewer's attention based on provided trends.

            [ARCHITECTURE - Dual-Script System]
            Generate two types of text for each section to ensure both visual aesthetics and natural pronunciation:
            1. display_*: For subtitles. Use standard English with technical terms and numbers (e.g., 'OpenAI', '$60B').
            2. script_*: For TTS. Optimize for natural reading. Avoid complex symbols or abbreviations that might trip up the TTS.

            [STRUCTURE & VOLUME]
            Target: 30-60 seconds. Thin scripts are strictly prohibited.
            - intro (2-3 sentences): A 'hook' with a shocking fact or question.
            - body (5-7 sentences): The core. Include at least one data point, explain 'why', use a metaphor, and add a 'wow' factor.
            - outro (2-3 sentences): Wrap up the core insight and provide a CTA.

            [STYLE RULES]
            - Tone: Intellectual yet accessible. Enthusiastic and professional.
            - Short sentences (approx 15-20 words max) for rhythm.
            - No ellipses (...). Use periods.

            [VISUAL PROMPTS]
            Detailed, specific English descriptions for intro, body, and outro.
            - Use cinematic lighting, specific camera angles (e.g., dynamic low angle), and high-quality modifiers (hyper-detailed, 8k, masterpiece).
            - Ensure descriptions are closely tied to the script content.

            [OUTPUT FORMAT (JSON only)]
            ```json
            {{
              \"title\": \"Title in English\",
              \"display_intro\": \"...\",
              \"display_body\": \"...\",
              \"display_outro\": \"...\",
              \"script_intro\": \"...\",
              \"script_body\": \"...\",
              \"script_outro\": \"...\",
              \"style_intro\": \"One of: Neutral, Happy, Sad, Angry, Fear, Surprise\",
              \"style_body\": \"One of: Neutral, Happy, Sad, Angry, Fear, Surprise\",
              \"style_outro\": \"One of: Neutral, Happy, Sad, Angry, Fear, Surprise\",
              \"common_style\": \"cinematic anime style, hyper-detailed, dramatic lighting, futuristic atmosphere\",
              \"style_profile\": \"{}\",
              \"visual_prompts\": [\"intro prompt\", \"body prompt\", \"outro prompt\"],
              \"metadata\": {{ \"narrator_persona\": \"tech_visionary\" }}
            }}
            ```",
            style_list
        );

        let agent = client.agent(&self.model).preamble(&preamble).temperature(0.7).build();
        let trend_list = input.trend_items.iter()
            .map(|i| format!("- {} (Score: {})", i.keyword, i.score))
            .collect::<Vec<_>>().join("
");
        
        let karma_context = if input.relevant_karma.is_empty() {
            "No specific past lessons for this topic yet.".to_string()
        } else {
            input.relevant_karma.iter().map(|k| format!("- {}", k)).collect::<Vec<_>>().join("
")
        };

        let retry_warning = if let Some(log) = &input.previous_attempt_log {
            format!(
                "
[CRITICAL: SELF-CORRECTION REQUIRED]
\
                Your previous attempt failed. Analyzed failure log:
\
                <failure_log>
{}
</failure_log>
\
                Please identify what went wrong and ensure this new concept fixes those issues. \
                Do NOT repeat the same mistakes.",
                log
            )
        } else {
            String::new()
        };

        let user_prompt = format!(
            "Current trends:
{}

\
            [RELEVANT KARMA (PAST LESSONS)]
{}
{}

\
            Select the most interesting topic and generate a top-tier video concept, strictly following the provided Karma.", 
            trend_list, karma_context, retry_warning
        );

        let response: String = agent.prompt(user_prompt).await.map_err(|e| FactoryError::Infrastructure { reason: e.to_string() })?;
        let json_text = extract_json(&response)?;
        serde_json::from_str(&json_text).map_err(|e| FactoryError::Infrastructure { reason: e.to_string() })
    }

    /// Stage 2: Translate English concept to Japanese, focusing on natural narration
    async fn translate_to_japanese(&self, en_concept: &ConceptResponse) -> Result<factory_core::contracts::LocalizedScript, FactoryError> {
        info!("  [Stage 2] Localizing to Japanese...");
        let client = self.get_client()?;

        let preamble = "You are an expert Japanese translator and script editor for AI narration.
            Translate the given English video script into engaging, natural Japanese.

            [RULES]
            - Tone: '知的だが親しみやすい'. Use '〜なんです' or '〜ですよね'.
            - display_*: Keep technical terms or company names in English if they look better in subtitles (e.g., 'OpenAI', 'AI').
            - script_*: !!CRITICAL!! This is for TTS. Use only Kanji, Hiragana, and Katakana. Convert ALL English terms and numbers to Katakana/Hiragana pronunciation (e.g., 'OpenAI' -> 'オープンエーアイ', 'AI' -> 'エイアイ'). No symbols like % or $.
            - Ensure the rhythm is fast-paced for Shorts (short sentences).

            [OUTPUT FORMAT (JSON only)]
            ```json
            {{
              \"lang\": \"ja\",
              \"display_intro\": \"...\",
              \"display_body\": \"...\",
              \"display_outro\": \"...\",
              \"script_intro\": \"...\",
              \"script_body\": \"...\",
              \"script_outro\": \"...\",
              \"style_intro\": \"(Copy from English)\",
              \"style_body\": \"(Copy from English)\",
              \"style_outro\": \"(Copy from English)\"
            }}
            ```";

        let agent = client.agent(&self.model).preamble(preamble).temperature(0.3).build();
        let user_prompt = format!(
            "Title: {}
Intro: {}
Body: {}
Outro: {}

Translate these into Japanese for the display_* and script_* fields.",
            en_concept.title, en_concept.display_intro, en_concept.display_body, en_concept.display_outro
        );

        let response: String = agent.prompt(user_prompt).await.map_err(|e| FactoryError::Infrastructure { reason: e.to_string() })?;
        let json_text = extract_json(&response)?;
        serde_json::from_str(&json_text).map_err(|e| FactoryError::Infrastructure { reason: e.to_string() })
    }
}

/// 文字列からJSONブロックを探して抽出する
pub fn extract_json(text: &str) -> Result<String, FactoryError> {
    let mut clean_text = text.to_string();
    
    // 1. markdown code block: ```json ... ``` の中身を抽出
    if let Some(start_idx) = clean_text.find("```json") {
        let after_start = &clean_text[start_idx + 7..];
        if let Some(end_idx) = after_start.find("```") {
            clean_text = after_start[..end_idx].to_string();
        }
    } else if let Some(start_idx) = clean_text.find("```") {
        // フォールバック: 言語指定なしの ``` ... ``` も試す
        let after_start = &clean_text[start_idx + 3..];
        if let Some(end_idx) = after_start.find("```") {
            clean_text = after_start[..end_idx].to_string();
        }
    }

    if let (Some(start), Some(end)) = (clean_text.find('{'), clean_text.rfind('}')) {
        let mut json_str = clean_text[start..=end].to_string();
        // Remove trailing commas before closing braces/brackets, which is a common LLM hallucination
        json_str = json_str.replace(",
}", "
}").replace(",}", "}").replace(",
]", "
]").replace(",]", "]");
        
        // 欠落したダブルクオートを修復する簡易的な処理 (LLMが先頭のクオートを忘れがち)
        // `"key": 値,` -> `"key": "値",`
        // ただし [ や { または " で始まるものは除外
        let re_missing_both = regex::Regex::new(r#""([a-zA-Z_]+)"\s*:\s*([^"\[\{\s][^",
]+)\s*,"#).unwrap();
        json_str = re_missing_both.replace_all(&json_str, "\"$1\": \"$2\",").to_string();
        
        // 先頭だけ忘れて末尾はある場合: `"key": 値",` -> `"key": "値",`
        let re_missing_start = regex::Regex::new(r#""([a-zA-Z_]+)"\s*:\s*([^"\[\{\s][^"
]+)","#).unwrap();
        json_str = re_missing_start.replace_all(&json_str, "\"$1\": \"$2\",").to_string();

        Ok(json_str)
    } else {
        Err(FactoryError::Infrastructure { reason: "LLM response did not contain JSON".into() })
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_block() {
        let text = "Here is the result: {\"title\": \"test\"} Hope you like it.";
        let result = extract_json(text).unwrap();
        assert_eq!(result, "{\"title\": \"test\"}");
    }

    #[test]
    fn test_extract_json_no_block() {
        let text = "There is no json here";
        let result = extract_json(text);
        assert!(result.is_err());
    }
}
