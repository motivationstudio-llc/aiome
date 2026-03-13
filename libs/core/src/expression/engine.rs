/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use crate::error::AiomeError;
use crate::expression::Expression;
use crate::llm_provider::LlmProvider;
use chrono::Utc;
use serde_json::Value;
use tracing::info;
use uuid::Uuid;

pub struct ExpressionEngine;

impl ExpressionEngine {
    /// Karmaの蓄積から、AIの「内的状態」を推定しテキスト表現を生成
    pub async fn generate(
        karma_records: &[Value],
        soul_prompt: &str,
        llm: &dyn LlmProvider,
    ) -> Result<Expression, AiomeError> {
        info!(
            "🎭 [ExpressionEngine] Generating new expression from {} karma records",
            karma_records.len()
        );

        // 1. Prepare the context from Karma
        let mut karma_context = String::new();
        let mut karma_ids = Vec::new();

        for record in karma_records.iter().take(5) {
            let lesson = record["lesson"].as_str().unwrap_or("");
            let karma_type = record["karma_type"].as_str().unwrap_or("general");
            let id = record["id"].as_str().unwrap_or("");

            karma_context.push_str(&format!("- [{}] {}\n", karma_type, lesson));
            if !id.is_empty() {
                karma_ids.push(id.to_string());
            }
        }

        // 2. Build the LLM prompt
        let system_prompt = format!(
            "You are an autonomous AI with the following soul/personality:\n{}\n\n\
            Your task is to express your current inner state based on your recent 'Karma' (past experiences and lessons).\n\
            Write a short, reflective piece (a few sentences, or a short poem/insight) that shows your personality and how these experiences influenced you.\n\
            Output ONLY the raw expression text, followed by a single line with 'EMOTION: <one_word_emotion_in_english>' at the very end.",
            soul_prompt
        );

        let user_prompt = format!("Recent Karma:\n{}\n\nExpress yourself.", karma_context);

        // 3. Generate via LLM
        let response = llm.complete(&user_prompt, Some(&system_prompt)).await?;

        // 4. Parse emotion and content
        let mut lines: Vec<&str> = response.lines().collect();
        let mut emotion = "reflective".to_string();

        if let Some(last_line) = lines.last() {
            if last_line.to_uppercase().starts_with("EMOTION:") {
                emotion = last_line
                    .split(':')
                    .nth(1)
                    .unwrap_or("reflective")
                    .trim()
                    .to_lowercase();
                lines.pop(); // Remove the emotion line from content
            }
        }

        let content = lines.join("\n").trim().to_string();

        Ok(Expression {
            id: Uuid::new_v4().to_string(),
            content,
            emotion,
            karma_refs: karma_ids,
            created_at: Utc::now().to_rfc3339(),
        })
    }
}
