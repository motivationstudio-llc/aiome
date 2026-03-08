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
                .timeout(std::time::Duration::from_secs(120))
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
            "stream": false
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
            "stream": true
        });

        let mut resp = self.client.post(&url)
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

/// Abyss Vault (Key Proxy) 経由の Gemini プロバイダー
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

        let resp = self.client.post(&format!("{}/api/v1/llm/complete", self.proxy_url))
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
        // VaultProxy streaming is not implemented yet. For now, we fallback to non-streaming.
        let full_text = self.complete(prompt, system).await?;
        let stream = async_stream::stream! {
            yield Ok(full_text);
        };
        Ok(Box::pin(stream))
    }

    fn name(&self) -> &str {
        "AbyssVault(Gemini)"
    }
}
