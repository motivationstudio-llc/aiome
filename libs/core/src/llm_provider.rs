/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

use crate::error::AiomeError;
use async_trait::async_trait;
use std::fmt::Debug;
// Unused imports removed.
use reqwest;
use serde_json;

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
        system: Option<&str>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AiomeError>> + Send>>, AiomeError> {
        let text = self.complete(prompt, system).await?;
        let s = async_stream::stream! {
            yield Ok(text);
        };
        Ok(Box::pin(s))
    }

    /// 接続テスト
    async fn test_connection(&self) -> Result<(), AiomeError>;

    /// プロバイダー名を取得（デバッグ用）
    fn name(&self) -> &str;
}

/// 埋め込み（Embedding）プロバイダーの共通インターフェース
#[async_trait]
pub trait EmbeddingProvider: Send + Sync + Debug {
    /// テキストをベクトルに変換
    /// is_query: trueの場合は検索クエリ用、falseの場合はドキュメント用として解釈する
    async fn embed(&self, text: &str, is_query: bool) -> Result<Vec<f32>, AiomeError>;

    /// 接続テスト
    async fn test_connection(&self) -> Result<(), AiomeError>;

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

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Ollama request failed: {}", e),
            })?;

        let body: serde_json::Value =
            resp.json().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("Ollama response parse failed: {}", e),
            })?;

        Ok(body["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }

    async fn stream_complete(
        &self,
        prompt: &str,
        system: Option<&str>,
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
        let mut resp = stream_client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Ollama stream request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            return Err(AiomeError::Infrastructure {
                reason: format!("Ollama stream error: {}", resp.status()),
            });
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

    async fn test_connection(&self) -> Result<(), AiomeError> {
        let url = format!("{}/api/tags", self.host);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Ollama connection test failed: {}", e),
            })?;

        if !resp.status().is_success() {
            return Err(AiomeError::Infrastructure {
                reason: format!("Ollama connection error: {}", resp.status()),
            });
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "Ollama"
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaProvider {
    async fn embed(&self, text: &str, _is_query: bool) -> Result<Vec<f32>, AiomeError> {
        let url = format!("{}/api/embeddings", self.host);
        let payload = serde_json::json!({
            "model": self.model,
            "prompt": text
        });

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Ollama embedding request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            return Err(AiomeError::Infrastructure {
                reason: format!("Ollama embedding error: {}", resp.status()),
            });
        }

        let body: serde_json::Value =
            resp.json().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("Ollama embedding parse failed: {}", e),
            })?;

        let embedding = body["embedding"]
            .as_array()
            .ok_or_else(|| AiomeError::Infrastructure {
                reason: "Ollama embedding missing in response".into(),
            })?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        Ok(embedding)
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        let url = format!("{}/api/tags", self.host);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Ollama embed connection test failed: {}", e),
            })?;
        if !resp.status().is_success() {
            return Err(AiomeError::Infrastructure {
                reason: "Ollama embed connection error".into(),
            });
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "Ollama(Embed)"
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

        let resp = self
            .client
            .post(format!("{}/api/v1/llm/complete", self.proxy_url))
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("VaultProxy request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            return Err(AiomeError::Infrastructure {
                reason: format!("VaultProxy returned error: {}", resp.status()),
            });
        }

        let body: serde_json::Value =
            resp.json().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("VaultProxy response parse failed: {}", e),
            })?;

        Ok(body["result"].as_str().unwrap_or("").to_string())
    }

    async fn stream_complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AiomeError>> + Send>>, AiomeError> {
        let payload = serde_json::json!({
            "caller_id": self.caller_id,
            "prompt": prompt,
            "system": system,
            "endpoint": "gemini"
        });

        let mut resp = self
            .client
            .post(format!("{}/api/v1/llm/stream", self.proxy_url))
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("VaultProxy stream request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            return Err(AiomeError::Infrastructure {
                reason: format!("VaultProxy returned error: {}", resp.status()),
            });
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

    async fn test_connection(&self) -> Result<(), AiomeError> {
        // Vault proxy connection test
        let url = format!("{}/api/v1/health", self.proxy_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("VaultProxy connection failed: {}", e),
            })?;
        if !resp.status().is_success() {
            return Err(AiomeError::Infrastructure {
                reason: "VaultProxy connection error".into(),
            });
        }
        Ok(())
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
        Self {
            client,
            api_key,
            model,
        }
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

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Gemini request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("Gemini error {}: {}", url, err_text),
            });
        }

        let body: serde_json::Value =
            resp.json().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("Gemini parse failed: {}", e),
            })?;

        Ok(body["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }

    async fn stream_complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AiomeError>> + Send>>, AiomeError> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
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

        let mut resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Gemini stream request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("Gemini stream error: {}", err_text),
            });
        }

        let stream = async_stream::stream! {
            let mut buffer = String::new();
            while let Ok(Some(chunk)) = resp.chunk().await {
                if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                    buffer.push_str(&text);
                    while let Some(idx) = buffer.find("data: ") {
                        let total_len;
                        let json_str_opt = {
                            let remainder = &buffer[idx + 6..];
                            if let Some(end_idx) = remainder.find("\n\n") {
                                total_len = Some(idx + 6 + end_idx + 2);
                                Some(remainder[..end_idx].to_string())
                            } else {
                                total_len = None;
                                None
                            }
                        };

                        if let (Some(t_len), Some(json_str)) = (total_len, json_str_opt) {
                            buffer = buffer[t_len..].to_string();
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                                if let Some(content) = json["candidates"][0]["content"]["parts"][0]["text"].as_str() {
                                    yield Ok(content.to_string());
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

    async fn test_connection(&self) -> Result<(), AiomeError> {
        self.complete("ping", None).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        "Gemini"
    }
}

#[async_trait]
impl EmbeddingProvider for GeminiProvider {
    async fn embed(&self, text: &str, _is_query: bool) -> Result<Vec<f32>, AiomeError> {
        let embedding_model = if self.model.contains("embed") {
            self.model.clone()
        } else {
            "gemini-embedding-001".to_string() // Fallback to standard embedding model
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:embedContent?key={}",
            embedding_model, self.api_key
        );

        let payload = serde_json::json!({
            "model": format!("models/{}", embedding_model),
            "content": {
                "parts": [{ "text": text }]
            }
        });

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Gemini embedding request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("Gemini embedding error: {}", err_text),
            });
        }

        let body: serde_json::Value =
            resp.json().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("Gemini embedding parse failed: {}", e),
            })?;

        let embedding = body["embedding"]["values"]
            .as_array()
            .ok_or_else(|| AiomeError::Infrastructure {
                reason: "Gemini embedding missing in response".into(),
            })?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        Ok(embedding)
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        self.embed("ping", false).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        "Gemini(Embed)"
    }
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
        Self {
            client,
            api_key,
            model,
        }
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

        let resp = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("OpenAI request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("OpenAI error: {}", err_text),
            });
        }

        let body: serde_json::Value =
            resp.json().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("OpenAI parse failed: {}", e),
            })?;

        Ok(body["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }

    async fn stream_complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AiomeError>> + Send>>, AiomeError> {
        let url = "https://api.openai.com/v1/chat/completions";
        let mut messages = Vec::new();

        if let Some(sys) = system {
            messages.push(serde_json::json!({ "role": "system", "content": sys }));
        }
        messages.push(serde_json::json!({ "role": "user", "content": prompt }));

        let payload = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": true
        });

        let mut resp = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("OpenAI stream request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("OpenAI stream error: {}", err_text),
            });
        }

        let stream = async_stream::stream! {
            let mut buffer = String::new();
            while let Ok(Some(chunk)) = resp.chunk().await {
                if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                    buffer.push_str(&text);
                    while let Some(idx) = buffer.find("data: ") {
                        let total_len;
                        let line_opt = {
                            let remainder = &buffer[idx + 6..];
                            if let Some(end_idx) = remainder.find('\n') {
                                total_len = Some(idx + 6 + end_idx + 1);
                                Some(remainder[..end_idx].trim().to_string())
                            } else {
                                total_len = None;
                                None
                            }
                        };

                        if let (Some(t_len), Some(line)) = (total_len, line_opt) {
                            buffer = buffer[t_len..].to_string();
                            if line == "[DONE]" { break; }

                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                                if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                                    yield Ok(content.to_string());
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

    async fn test_connection(&self) -> Result<(), AiomeError> {
        self.complete("ping", None).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        "OpenAI"
    }
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
        Self {
            client,
            api_key,
            model,
        }
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

        let resp = self
            .client
            .post(url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Claude request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("Claude error: {}", err_text),
            });
        }

        let body: serde_json::Value =
            resp.json().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("Claude parse failed: {}", e),
            })?;

        Ok(body["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }

    async fn stream_complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AiomeError>> + Send>>, AiomeError> {
        let url = "https://api.anthropic.com/v1/messages";

        let payload = serde_json::json!({
            "model": self.model,
            "max_tokens": 4096,
            "system": system,
            "messages": [
                { "role": "user", "content": prompt }
            ],
            "stream": true
        });

        let mut resp = self
            .client
            .post(url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Claude stream request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("Claude stream error: {}", err_text),
            });
        }

        let stream = async_stream::stream! {
            let mut buffer = String::new();
            while let Ok(Some(chunk)) = resp.chunk().await {
                if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                    buffer.push_str(&text);
                    while let Some(idx) = buffer.find("data: ") {
                        let total_len;
                        let line_opt = {
                            let remainder = &buffer[idx + 6..];
                            if let Some(end_idx) = remainder.find('\n') {
                                total_len = Some(idx + 6 + end_idx + 1);
                                Some(remainder[..end_idx].trim().to_string())
                            } else {
                                total_len = None;
                                None
                            }
                        };

                        if let (Some(t_len), Some(line)) = (total_len, line_opt) {
                            buffer = buffer[t_len..].to_string();
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                                if let Some(content) = json["delta"]["text"].as_str() {
                                    yield Ok(content.to_string());
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

    async fn test_connection(&self) -> Result<(), AiomeError> {
        self.complete("ping", None).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        "Claude"
    }
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
        Self {
            client,
            host,
            model,
        }
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

        let resp = self
            .client
            .post(&url)
            .header("Authorization", "Bearer lm-studio")
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("LM Studio request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("LM Studio error: {}", err_text),
            });
        }

        let body: serde_json::Value =
            resp.json().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("LM Studio parse failed: {}", e),
            })?;

        Ok(body["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }

    async fn stream_complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AiomeError>> + Send>>, AiomeError> {
        let url = format!("{}/v1/chat/completions", self.host.trim_end_matches('/'));
        let mut messages = Vec::new();

        if let Some(sys) = system {
            messages.push(serde_json::json!({ "role": "system", "content": sys }));
        }
        messages.push(serde_json::json!({ "role": "user", "content": prompt }));

        let payload = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "temperature": 0.7,
            "stream": true
        });

        let mut resp = self
            .client
            .post(&url)
            .header("Authorization", "Bearer lm-studio")
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("LM Studio stream request failed: {}", e),
            })?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("LM Studio stream error: {}", err_text),
            });
        }

        let stream = async_stream::stream! {
            let mut buffer = String::new();
            while let Ok(Some(chunk)) = resp.chunk().await {
                if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                    buffer.push_str(&text);
                    while let Some(idx) = buffer.find("data: ") {
                        let total_len;
                        let line_opt = {
                            let remainder = &buffer[idx + 6..];
                            if let Some(end_idx) = remainder.find('\n') {
                                total_len = Some(idx + 6 + end_idx + 1);
                                Some(remainder[..end_idx].trim().to_string())
                            } else {
                                total_len = None;
                                None
                            }
                        };

                        if let (Some(t_len), Some(line)) = (total_len, line_opt) {
                            buffer = buffer[t_len..].to_string();
                            if line == "[DONE]" { break; }

                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                                if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                                    yield Ok(content.to_string());
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

    async fn test_connection(&self) -> Result<(), AiomeError> {
        self.complete("ping", None).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        "LMStudio"
    }
}

/// Ruri-v3 ローカル Embedding プロバイダー
/// Python サイドカー (tools/ruri-embed-server) 経由で ruri-v3-310m を利用
#[derive(Debug, Clone)]
pub struct RuriProvider {
    client: reqwest::Client,
    base_url: String,
}

impl RuriProvider {
    pub fn new(client: reqwest::Client, base_url: String) -> Self {
        Self { client, base_url }
    }
}

#[async_trait]
impl EmbeddingProvider for RuriProvider {
    async fn embed(&self, text: &str, is_query: bool) -> Result<Vec<f32>, AiomeError> {
        if text.trim().is_empty() {
            return Err(AiomeError::Infrastructure {
                reason: "Cannot generate embedding for empty text".into(),
            });
        }

        let url = format!("{}/embed", self.base_url);
        let mode = if is_query { "query" } else { "document" };
        let payload = serde_json::json!({
            "text": text,
            "mode": mode
        });

        let resp = self.client.post(&url)
            .timeout(std::time::Duration::from_secs(30))
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AiomeError::Infrastructure {
                        reason: format!("Ruri embedding timed out after 30s ({})", self.base_url)
                    }
                } else {
                    AiomeError::Infrastructure {
                        reason: format!("Ruri embedding request failed (is ruri-embed-server running on {}?): {}", self.base_url, e)
                    }
                }
            })?;

        if !resp.status().is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(AiomeError::Infrastructure {
                reason: format!("Ruri embedding error: {}", err_text),
            });
        }

        let body: serde_json::Value =
            resp.json().await.map_err(|e| AiomeError::Infrastructure {
                reason: format!("Ruri embedding parse failed: {}", e),
            })?;

        let embedding = body["embedding"]
            .as_array()
            .ok_or_else(|| AiomeError::Infrastructure {
                reason: "Ruri embedding missing in response".into(),
            })?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        Ok(embedding)
    }

    async fn test_connection(&self) -> Result<(), AiomeError> {
        let url = format!("{}/health", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Ruri connection failed: {}", e),
            })?;
        if !resp.status().is_success() {
            return Err(AiomeError::Infrastructure {
                reason: "Ruri connection error".into(),
            });
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "Ruri-v3(Embed)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_provider_initialization_and_names() {
        let client = reqwest::Client::new();

        let ollama =
            OllamaProvider::new("http://localhost:11434".to_string(), "llama3".to_string());
        assert_eq!(LlmProvider::name(&ollama), "Ollama");

        let gemini = GeminiProvider::new(client.clone(), "key".to_string(), "gemini".to_string());
        assert_eq!(LlmProvider::name(&gemini), "Gemini");

        let openai = OpenAiProvider::new(client.clone(), "key".to_string(), "gpt-4".to_string());
        assert_eq!(openai.name(), "OpenAI");

        let claude = ClaudeProvider::new(client.clone(), "key".to_string(), "claude".to_string());
        assert_eq!(claude.name(), "Claude");

        let lmstudio = LmStudioProvider::new(
            client.clone(),
            "http://localhost:1234".to_string(),
            "local".to_string(),
        );
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
