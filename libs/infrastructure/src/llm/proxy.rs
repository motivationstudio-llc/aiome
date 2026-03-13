/*
 * Aiome - The Autonomous AI Operating System
 */

use async_trait::async_trait;
use aiome_core::llm_provider::LlmProvider;
use aiome_core::error::AiomeError;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct ProxyLlmProvider {
    pub proxy_url: String,
    pub endpoint_tag: String,
    pub caller_id: String,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct ProxyRequest {
    caller_id: String,
    prompt: String,
    system: Option<String>,
    endpoint: String,
}

#[derive(Deserialize)]
struct ProxyResponse {
    result: String,
}

impl ProxyLlmProvider {
    pub fn new(proxy_url: String, endpoint_tag: String, caller_id: String) -> Self {
        Self {
            proxy_url,
            endpoint_tag,
            caller_id,
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
}

#[async_trait]
impl LlmProvider for ProxyLlmProvider {
    async fn complete(&self, prompt: &str, system: Option<&str>) -> Result<String, AiomeError> {
        let url = format!("{}/api/v1/llm/complete", self.proxy_url);
        
        let payload = ProxyRequest {
            caller_id: self.caller_id.clone(),
            prompt: prompt.to_string(),
            system: system.map(|s| s.to_string()),
            endpoint: self.endpoint_tag.clone(),
        };

        let res = self.client.post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        if !res.status().is_success() {
            return Err(AiomeError::Infrastructure { 
                reason: format!("KeyProxy returned error status: {}", res.status()) 
            });
        }

        let body: ProxyResponse = res.json().await
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        Ok(body.result)
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        self.complete("ping", None).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        "KeyProxy"
    }
}

#[async_trait]
impl aiome_core::llm_provider::EmbeddingProvider for ProxyLlmProvider {
    async fn embed(&self, text: &str, _is_query: bool) -> Result<Vec<f32>, AiomeError> {
        let url = format!("{}/api/v1/llm/embed", self.proxy_url);
        
        let payload = ProxyRequest {
            caller_id: self.caller_id.clone(),
            prompt: text.to_string(),
            system: None,
            endpoint: "gemini-embed".to_string(), 
        };

        let res = self.client.post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        if !res.status().is_success() {
            return Err(AiomeError::Infrastructure { 
                reason: format!("KeyProxy (Embed) error: {}", res.status()) 
            });
        }

        #[derive(Deserialize)]
        struct EmbedRes { embedding: Vec<f32> }
        let body: EmbedRes = res.json().await
            .map_err(|e| AiomeError::Infrastructure { reason: e.to_string() })?;

        Ok(body.embedding)
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        self.embed("ping", false).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        "KeyProxy(Embed)"
    }
}
