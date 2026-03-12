/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 * 
 * Licensed under the Elastic License 2.0 (ELv2).
 * You may not provide the software to third parties as a hosted or managed service, 
 * where the service provides users with access to any substantial set of the features 
 * or functionality of the software.
 */

use async_trait::async_trait;
use crate::error::AiomeError;
use std::fmt::Debug;
// Unused imports removed.
use serde_json;
use reqwest;

use std::pin::Pin;
use tokio_stream::Stream;

/// LLMプロバイダーの共通インターフェース
#[async_trait]
pub trait LlmProvider: Send + Sync + Debug {
    /// テキスト生成リクエスト
    async fn complete(&self, prompt: &str, system: Option<&str>) -> Result<String, AiomeError>;
    
    /// ストリーミング生成リクエスト
    async fn stream_complete(
        &self, 
        prompt: &str, 
        system: Option<&str>
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AiomeError>> + Send>>, AiomeError> {
        let text = self.complete(prompt, system).await?;
        let s = async_stream::stream! {
            yield Ok(text);
        };
        Ok(Box::pin(s))
    }

    /// プロバイダー名を取得（デバッグ用）
    fn name(&self) -> &str;
}

/// 埋め込み（Embedding）プロバイダーの共通インターフェース
#[async_trait]
pub trait EmbeddingProvider: Send + Sync + Debug {
    /// テキストをベクトルに変換
    async fn embed(&self, text: &str) -> Result<Vec<f32>, AiomeError>;
    fn name(&self) -> &str;
}

// --- 実装 ---

/// Ollama (ローカルLLM) プロバイダー
#[derive(Debug, Clone)]
pub struct OllamaProvider {
    host: String,
    model: String,
    client: reqwest::Client,
}

impl OllamaProvider {
    pub fn new(host: String, model: String) -> Self {
        Self {
            host,
            model,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(45))
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn complete(&self, prompt: &str, system: Option<&str>) -> Result<String, AiomeError> {
        let url = format!("{}/api/chat", self.host);
        let mut messages = Vec::new();

        if let Some(sys) = system {
            messages.push(serde_json::json!({
                "role": "system",
                "content": sys
            }));
        }

        messages.push(serde_json::json!({
            "role": "user",
            "content": prompt
        }));

        let payload = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
            "think": false,
            "options": {
                "num_predict": 4096,
                "temperature": 0.7
            }
        });

        let resp = self.client.post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Ollama request failed: {}", e) })?;

        let body: serde_json::Value = resp.json().await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Ollama response parse failed: {}", e) })?;

        Ok(body["message"]["content"].as_str().unwrap_or("").to_string())
    }

    async fn stream_complete(
        &self,
        prompt: &str,
        system: Option<&str>
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AiomeError>> + Send>>, AiomeError> {
        let url = format!("{}/api/chat", self.host);
        let mut messages = Vec::new();

        if let Some(sys) = system {
            messages.push(serde_json::json!({
                "role": "system",
                "content": sys
            }));
        }

        messages.push(serde_json::json!({
            "role": "user",
            "content": prompt
        }));

        let payload = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
            "think": false,
            "options": {
                "num_predict": 4096,
                "temperature": 0.7
            }
        });

        // Use a longer timeout for streaming since data comes in chunks over time
        let stream_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .unwrap_or_default();
        let mut resp = stream_client.post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Ollama stream request failed: {}", e) })?;

        if !resp.status().is_success() {
            return Err(AiomeError::Infrastructure { reason: format!("Ollama stream error: {}", resp.status()) });
        }

        let stream = async_stream::stream! {
            let mut incomplete_chunk = String::new();
            while let Ok(Some(chunk)) = resp.chunk().await {
                if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                    incomplete_chunk.push_str(&text);
                    
                    // Ollama streams NDJSON. We split by newline.
                    while let Some(idx) = incomplete_chunk.find('\n') {
                        let line = incomplete_chunk[..idx].to_string();
                        incomplete_chunk = incomplete_chunk[idx+1..].to_string();

                        if line.trim().is_empty() {
                            continue;
                        }

                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                            if let Some(content) = json.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
                                yield Ok(content.to_string());
                            }
                        }
                    }
                } else {
                    yield Err(AiomeError::Infrastructure { reason: "Invalid UTF-8 in Ollama stream chunk".into() });
                }
            }
        };

        Ok(Box::pin(stream))
    }

    fn name(&self) -> &str {
        "Ollama"
    }
}

/// Abyss Vault (Key Proxy) 経由の Gemini プロバイダー (DEPRECATED: Direct GeminiProvider推奨)
#[derive(Debug, Clone)]
pub struct AbyssVaultProvider {
    proxy_url: String,
    caller_id: String,
    client: reqwest::Client,
}

impl AbyssVaultProvider {
    pub fn new(proxy_url: String, caller_id: String) -> Self {
        Self {
            proxy_url,
            caller_id,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmProvider for AbyssVaultProvider {
    async fn complete(&self, prompt: &str, system: Option<&str>) -> Result<String, AiomeError> {
        let payload = serde_json::json!({
            "caller_id": self.caller_id,
            "prompt": prompt,
            "system": system,
            "endpoint": "gemini"
        });

        let resp = self.client.post(format!("{}/api/v1/llm/complete", self.proxy_url))
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("VaultProxy request failed: {}", e) })?;

        if !resp.status().is_success() {
            return Err(AiomeError::Infrastructure { reason: format!("VaultProxy returned error: {}", resp.status()) });
        }

        let body: serde_json::Value = resp.json().await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("VaultProxy response parse failed: {}", e) })?;

        Ok(body["result"].as_str().unwrap_or("").to_string())
    }

    async fn stream_complete(
        &self,
        prompt: &str,
        system: Option<&str>
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AiomeError>> + Send>>, AiomeError> {
        let payload = serde_json::json!({
            "caller_id": self.caller_id,
            "prompt": prompt,
            "system": system,
            "endpoint": "gemini"
        });

        let mut resp = self.client.post(format!("{}/api/v1/llm/stream", self.proxy_url))
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("VaultProxy stream request failed: {}", e) })?;

        if !resp.status().is_success() {
            return Err(AiomeError::Infrastructure { reason: format!("VaultProxy returned error: {}", resp.status()) });
        }

        let stream = async_stream::stream! {
            let mut buffer = String::new();
            while let Ok(Some(chunk)) = resp.chunk().await {
                if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                    buffer.push_str(&text);
                    
                    while let Some(data_idx) = buffer.find("data: ") {
                        let remainder = &buffer[data_idx + 6..];
                        if let Some(end_idx) = remainder.find("\n\n") {
                            let json_str = remainder[..end_idx].to_string();
                            let total_len = data_idx + 6 + end_idx + 2;
                            buffer = buffer[total_len..].to_string();

                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                                if let Some(text_chunk) = json["candidates"][0]["content"]["parts"][0]["text"].as_str() {
                                    yield Ok(text_chunk.to_string());
                                }
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    fn name(&self) -> &str {
        "AbyssVault(Gemini)"
    }
}

// --- Cloud Provider Implementations ---

/// Google Gemini Provider
#[derive(Debug, Clone)]
pub struct GeminiProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl GeminiProvider {
    pub fn new(client: reqwest::Client, api_key: String, model: String) -> Self {
        Self { client, api_key, model }
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn complete(&self, prompt: &str, system: Option<&str>) -> Result<String, AiomeError> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let payload = serde_json::json!({
            "contents": [{
                "parts": [{ "text": prompt }]
            }],
            "system_instruction": system.map(|s| {
                serde_json::json!({ "parts": [{ "text": s }] })
            })
        });

        let resp = self.client.post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Gemini request failed: {}", e) })?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure { reason: format!("Gemini error {}: {}", url, err_text) });
        }

        let body: serde_json::Value = resp.json().await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Gemini parse failed: {}", e) })?;

        Ok(body["candidates"][0]["content"]["parts"][0]["text"].as_str().unwrap_or("").to_string())
    }

    fn name(&self) -> &str { "Gemini" }
}

/// OpenAI Chat Completions Provider
#[derive(Debug, Clone)]
pub struct OpenAiProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl OpenAiProvider {
    pub fn new(client: reqwest::Client, api_key: String, model: String) -> Self {
        Self { client, api_key, model }
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn complete(&self, prompt: &str, system: Option<&str>) -> Result<String, AiomeError> {
        let url = "https://api.openai.com/v1/chat/completions";
        let mut messages = Vec::new();

        if let Some(sys) = system {
            messages.push(serde_json::json!({ "role": "system", "content": sys }));
        }
        messages.push(serde_json::json!({ "role": "user", "content": prompt }));

        let payload = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "temperature": 0.7
        });

        let resp = self.client.post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("OpenAI request failed: {}", e) })?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure { reason: format!("OpenAI error: {}", err_text) });
        }

        let body: serde_json::Value = resp.json().await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("OpenAI parse failed: {}", e) })?;

        Ok(body["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string())
    }

    fn name(&self) -> &str { "OpenAI" }
}

/// Anthropic Claude Provider (Messages API)
#[derive(Debug, Clone)]
pub struct ClaudeProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl ClaudeProvider {
    pub fn new(client: reqwest::Client, api_key: String, model: String) -> Self {
        Self { client, api_key, model }
    }
}

#[async_trait]
impl LlmProvider for ClaudeProvider {
    async fn complete(&self, prompt: &str, system: Option<&str>) -> Result<String, AiomeError> {
        let url = "https://api.anthropic.com/v1/messages";

        let payload = serde_json::json!({
            "model": self.model,
            "max_tokens": 4096,
            "system": system,
            "messages": [
                { "role": "user", "content": prompt }
            ]
        });

        let resp = self.client.post(url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Claude request failed: {}", e) })?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure { reason: format!("Claude error: {}", err_text) });
        }

        let body: serde_json::Value = resp.json().await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Claude parse failed: {}", e) })?;

        Ok(body["content"][0]["text"].as_str().unwrap_or("").to_string())
    }

    fn name(&self) -> &str { "Claude" }
}

/// LM Studio Provider (OpenAI-compatible local server)
#[derive(Debug, Clone)]
pub struct LmStudioProvider {
    client: reqwest::Client,
    host: String,
    model: String,
}

impl LmStudioProvider {
    pub fn new(client: reqwest::Client, host: String, model: String) -> Self {
        Self { client, host, model }
    }
}

#[async_trait]
impl LlmProvider for LmStudioProvider {
    async fn complete(&self, prompt: &str, system: Option<&str>) -> Result<String, AiomeError> {
        let url = format!("{}/v1/chat/completions", self.host.trim_end_matches('/'));
        let mut messages = Vec::new();

        if let Some(sys) = system {
            messages.push(serde_json::json!({ "role": "system", "content": sys }));
        }
        messages.push(serde_json::json!({ "role": "user", "content": prompt }));

        let payload = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "temperature": 0.7
        });

        let resp = self.client.post(&url)
            .header("Authorization", "Bearer lm-studio")
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("LM Studio request failed: {}", e) })?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure { reason: format!("LM Studio error: {}", err_text) });
        }

        let body: serde_json::Value = resp.json().await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("LM Studio parse failed: {}", e) })?;

        Ok(body["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string())
    }

    fn name(&self) -> &str { "LMStudio" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_provider_initialization_and_names() {
        let client = reqwest::Client::new();
        
        let ollama = OllamaProvider::new("http://localhost:11434".to_string(), "llama3".to_string());
        assert_eq!(ollama.name(), "Ollama");
        
        let gemini = GeminiProvider::new(client.clone(), "key".to_string(), "gemini".to_string());
        assert_eq!(gemini.name(), "Gemini");

        let openai = OpenAiProvider::new(client.clone(), "key".to_string(), "gpt-4".to_string());
        assert_eq!(openai.name(), "OpenAI");

        let claude = ClaudeProvider::new(client.clone(), "key".to_string(), "claude".to_string());
        assert_eq!(claude.name(), "Claude");
        
        let lmstudio = LmStudioProvider::new(client.clone(), "http://localhost:1234".to_string(), "local".to_string());
        assert_eq!(lmstudio.name(), "LMStudio");
    }

    #[tokio::test]
    async fn test_lmstudio_complete_success() {
        let mock_server = MockServer::start().await;
        let mock_response = serde_json::json!({
            "choices": [{
                "message": {
                    "content": "Hello from mock LM Studio"
                }
            }]
        });

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let provider = LmStudioProvider::new(client, mock_server.uri(), "test-model".to_string());
        
        let result = provider.complete("Say hello", Some("System prompt")).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello from mock LM Studio");
    }
}
