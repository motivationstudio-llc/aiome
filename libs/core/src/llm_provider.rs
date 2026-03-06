/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 * 
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 */

use async_trait::async_trait;
use crate::error::AiomeError;
use std::fmt::Debug;
// Unused imports removed.
use serde_json;
use reqwest;

/// LLMプロバイダーの共通インターフェース
#[async_trait]
pub trait LlmProvider: Send + Sync + Debug {
    /// テキスト生成リクエスト
    async fn complete(&self, prompt: &str, system: Option<&str>) -> Result<String, AiomeError>;
    
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
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn complete(&self, prompt: &str, system: Option<&str>) -> Result<String, AiomeError> {
        let url = format!("{}/api/generate", self.host);
        let payload = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "system": system,
            "stream": false
        });

        let resp = self.client.post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Ollama request failed: {}", e) })?;

        let body: serde_json::Value = resp.json().await
            .map_err(|e| AiomeError::Infrastructure { reason: format!("Ollama response parse failed: {}", e) })?;

        Ok(body["response"].as_str().unwrap_or("").to_string())
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

    fn name(&self) -> &str {
        "AbyssVault(Gemini)"
    }
}
