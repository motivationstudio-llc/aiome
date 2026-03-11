/*
 * Aiome - The Autonomous AI Operating System
 */

use async_trait::async_trait;
use aiome_core::llm_provider::LlmProvider;
use aiome_core::error::AiomeError;
use serde_json::json;

#[derive(Debug)]
pub struct OllamaProvider {
    pub base_url: String,
    pub model: String,
    client: reqwest::Client,
}

impl OllamaProvider {
    pub fn new(base_url: String, model: String) -> Self {
        Self {
            base_url,
            model,
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn complete(&self, prompt: &str, system: Option<&str>) -> Result<String, AiomeError> {
        let url = format!("{}/api/chat", self.base_url);
        
        let mut messages = Vec::new();
        if let Some(sys) = system {
            messages.push(json!({
                "role": "system",
                "content": sys
            }));
        }
        messages.push(json!({
            "role": "user",
            "content": prompt
        }));

        let payload = json!({
            "model": self.model,
            "messages": messages,
            "stream": false
        });

        let res = self.client.post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        let body: serde_json::Value = res.json().await
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        Ok(body["message"]["content"].as_str().unwrap_or("").to_string())
    }

    fn name(&self) -> &str {
        "Ollama"
    }
}
