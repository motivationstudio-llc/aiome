/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

use crate::error::AppError;
use crate::{auth::Authenticated, AppState};
use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, ToSocketAddrs};
use tracing::{error, info, warn};
use url::Url;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateSettingsRequest {
    pub key: String,
    pub value: String,
    pub category: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct TestConnectionRequest {
    pub service: String, // "ollama", "discord", "telegram"
    pub url: String,
    pub token: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub message: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/settings",
    responses(
        (status = 200, description = "List all settings", body = [aiome_core::contracts::SystemSetting]),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = []))
)]
pub async fn get_settings(
    State(state): State<AppState>,
    _auth: Authenticated,
) -> Result<Json<Vec<aiome_core::contracts::SystemSetting>>, AppError> {
    let settings = state.job_queue.fetch_all_settings().await?;

    // Mask secrets
    let mut masked = settings;
    for s in &mut masked {
        if s.is_secret {
            s.value = "••••••••".to_string();
        }
    }
    Ok(Json(masked))
}

#[utoipa::path(
    put,
    path = "/api/v1/settings",
    request_body = UpdateSettingsRequest,
    responses(
        (status = 200, description = "Setting updated successfully"),
        (status = 400, description = "Invalid request or unauthorized key"),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = []))
)]
pub async fn update_setting(
    State(state): State<AppState>,
    _auth: Authenticated,
    Json(payload): Json<UpdateSettingsRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // 1. Key whitelist check
    let allowed_keys = [
        "ollama_host",
        "ollama_model",
        "llm_provider",
        "llm_api_key",
        "llm_model",
        "lm_studio_host",
        "bg_llm_provider",
        "bg_llm_model",
        "bg_llm_api_key",
        "discord_chat_channel_id",
        "discord_command_channel_id",
        "discord_log_channel_id",
        "telegram_chat_id",
        "watchtower_enabled",
        "enforce_guardrail",
        "log_level",
        "node_id",
        "samsara_hub_url",
        "allowed_origins",
        "ai_name",
        "ai_motto",
        "ai_vrm_url",
    ];

    if !allowed_keys.contains(&payload.key.as_str()) {
        warn!(
            "🚨 [Security] Unauthorized settings key attempt: {}",
            payload.key
        );
        return Err(aiome_core::error::AiomeError::SecurityViolation {
            reason: "Unauthorized setting key".to_string(),
        }
        .into());
    }

    // 2. Category validation
    let allowed_categories = ["llm", "channel", "system", "security", "cors", "identity"];
    if !allowed_categories.contains(&payload.category.as_str()) {
        return Err(
            aiome_core::error::AiomeError::RemoteServiceExecutionFailed {
                reason: "Invalid category".to_string(),
            }
            .into(),
        );
    }

    // 3. Value length limit (DoS protection)
    if payload.value.len() > 1024 {
        return Err(
            aiome_core::error::AiomeError::RemoteServiceExecutionFailed {
                reason: "Value too long (max 1024 chars)".to_string(),
            }
            .into(),
        );
    }

    // 4. Server-side is_secret determination
    let secrets = [
        "ollama_host",
        "discord_token",
        "telegram_token",
        "api_server_secret",
        "llm_api_key",
    ];
    let is_secret = secrets.contains(&payload.key.as_str());

    // 5. Audit Logging
    info!(
        "🔧 [Settings] Audit: Key '{}' updated (category: {}, secret: {})",
        payload.key, payload.category, is_secret
    );

    state
        .job_queue
        .update_setting(&payload.key, &payload.value, &payload.category, is_secret)
        .await?;

    Ok(Json(serde_json::json!({"status": "ok"})))
}

#[utoipa::path(
    post,
    path = "/api/v1/settings/test",
    request_body = TestConnectionRequest,
    responses(
        (status = 200, description = "Connection test completed", body = TestConnectionResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = []))
)]
pub async fn test_connection(
    _state: State<AppState>,
    _auth: Authenticated,
    Json(payload): Json<TestConnectionRequest>,
) -> Result<Json<TestConnectionResponse>, AppError> {
    // SSRF Protection
    if let Err(e) = validate_safe_url(&payload.url) {
        return Ok(Json(TestConnectionResponse {
            success: false,
            message: format!("SSRF Blocked: {}", e),
        }));
    }

    let res = match payload.service.as_str() {
        "ollama" => test_ollama(&payload.url, payload.model.as_deref()).await,
        "gemini" | "openai" | "anthropic" => {
            test_cloud_connection(
                &payload.service,
                payload.token.as_deref(),
                payload.model.as_deref(),
            )
            .await
        }
        _ => Json(TestConnectionResponse {
            success: false,
            message: format!("Service '{}' testing not implemented yet", payload.service),
        }),
    };

    Ok(res)
}

async fn test_cloud_connection(
    service: &str,
    token: Option<&str>,
    _model: Option<&str>,
) -> Json<TestConnectionResponse> {
    let Some(token) = token else {
        return Json(TestConnectionResponse {
            success: false,
            message: format!("API Key is required for {}", service),
        });
    };
    if token.is_empty() {
        return Json(TestConnectionResponse {
            success: false,
            message: format!("API Key is required for {}", service),
        });
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    match service {
        "gemini" => {
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models?key={}",
                token
            );
            match client.get(url).send().await {
                Ok(res) if res.status().is_success() => Json(TestConnectionResponse {
                    success: true,
                    message: "Gemini connection verified.".to_string(),
                }),
                Ok(res) => Json(TestConnectionResponse {
                    success: false,
                    message: format!("Gemini error: Status {}", res.status()),
                }),
                Err(e) => Json(TestConnectionResponse {
                    success: false,
                    message: format!("Gemini connection failed: {}", e),
                }),
            }
        }
        "openai" => {
            match client
                .get("https://api.openai.com/v1/models")
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await
            {
                Ok(res) if res.status().is_success() => Json(TestConnectionResponse {
                    success: true,
                    message: "OpenAI connection verified.".to_string(),
                }),
                Ok(res) => Json(TestConnectionResponse {
                    success: false,
                    message: format!("OpenAI error: Status {}", res.status()),
                }),
                Err(e) => Json(TestConnectionResponse {
                    success: false,
                    message: format!("OpenAI connection failed: {}", e),
                }),
            }
        }
        "claude" => {
            match client
                .get("https://api.anthropic.com/v1/models")
                .header("x-api-key", token)
                .header("anthropic-version", "2023-06-01")
                .send()
                .await
            {
                Ok(res) if res.status().is_success() => Json(TestConnectionResponse {
                    success: true,
                    message: "Claude connection verified.".to_string(),
                }),
                Ok(res) => Json(TestConnectionResponse {
                    success: false,
                    message: format!("Claude error: Status {}", res.status()),
                }),
                Err(e) => Json(TestConnectionResponse {
                    success: false,
                    message: format!("Claude connection failed: {}", e),
                }),
            }
        }
        _ => Json(TestConnectionResponse {
            success: false,
            message: format!("Prover '{}' testing not fully implemented", service),
        }),
    }
}

fn validate_safe_url(url_str: &str) -> Result<(), String> {
    let parsed = Url::parse(url_str).map_err(|e| e.to_string())?;
    let host = parsed.host_str().ok_or("No host segment")?;

    // Allow loopback for local AI services (Ollama etc.)
    if host == "localhost" || host == "127.0.0.1" || host == "::1" {
        return Ok(());
    }

    // Explicitly block Cloud Metadata IP (AWS/GCP/Azure)
    if host == "169.254.169.254" {
        return Err("Access to cloud metadata is forbidden".to_string());
    }

    // Resolve and check for private IPs
    if let Ok(addrs) = (host, 0).to_socket_addrs() {
        for addr in addrs {
            let ip = addr.ip();
            if is_private_ip(ip) {
                return Err(format!(
                    "Access to private network address is forbidden: {}",
                    ip
                ));
            }
        }
    }

    Ok(())
}

fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_link_local()
                || v4.is_loopback()
                || v4.is_unspecified()
                || v4.octets() == [169, 254, 169, 254]
        }
        IpAddr::V6(v6) => {
            // Check for Link-Local (fe80::/10), Unique Local (fc00::/7), etc.
            let segments = v6.segments();
            let is_link_local = (segments[0] & 0xffc0) == 0xfe80;
            let is_unique_local = (segments[0] & 0xfe00) == 0xfc00;
            is_link_local || is_unique_local || v6.is_loopback() || v6.is_unspecified()
        }
    }
}

async fn test_ollama(host: &str, model: Option<&str>) -> Json<TestConnectionResponse> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let url = if host.ends_with('/') {
        format!("{}api/tags", host)
    } else {
        format!("{}/api/tags", host)
    };

    match client.get(&url).send().await {
        Ok(res) if res.status().is_success() => {
            if let Ok(json) = res.json::<serde_json::Value>().await {
                if let Some(model_name) = model {
                    let models = json.get("models").and_then(|m| m.as_array());
                    if let Some(models) = models {
                        let found = models
                            .iter()
                            .any(|m| m.get("name").and_then(|n| n.as_str()) == Some(model_name));
                        if found {
                            Json(TestConnectionResponse {
                                success: true,
                                message: format!(
                                    "Ollama connection OK. Model '{}' found.",
                                    model_name
                                ),
                            })
                        } else {
                            Json(TestConnectionResponse {
                                success: false,
                                message: format!(
                                    "Ollama connection OK, but model '{}' was not found.",
                                    model_name
                                ),
                            })
                        }
                    } else {
                        Json(TestConnectionResponse {
                            success: true,
                            message: "Ollama connection OK (model list empty).".to_string(),
                        })
                    }
                } else {
                    Json(TestConnectionResponse {
                        success: true,
                        message: "Ollama connection OK.".to_string(),
                    })
                }
            } else {
                Json(TestConnectionResponse {
                    success: false,
                    message: "Ollama responded but failed to parse JSON.".to_string(),
                })
            }
        }
        Ok(res) => Json(TestConnectionResponse {
            success: false,
            message: format!("Ollama returned error status: {}", res.status()),
        }),
        Err(e) => Json(TestConnectionResponse {
            success: false,
            message: format!("Failed to connect to Ollama: {}", e),
        }),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/ollama/models",
    responses(
        (status = 200, description = "List available Ollama models", body = serde_json::Value),
        (status = 401, description = "Unauthorized")
    ),
    security(("api_key" = []))
)]
pub async fn get_ollama_models(
    State(state): State<AppState>,
    _auth: Authenticated,
) -> Result<Json<serde_json::Value>, AppError> {
    let host = state
        .job_queue
        .get_setting_value("ollama_host")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| {
            std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string())
        });
    let url = format!("{}/api/tags", host.trim_end_matches('/'));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    let res = client.get(&url).send().await.map_err(|e| {
        aiome_core::error::AiomeError::RemoteServiceError {
            url: url.clone(),
            source: e.into(),
        }
    })?;

    if res.status().is_success() {
        let json = res.json::<serde_json::Value>().await.map_err(|e| {
            aiome_core::error::AiomeError::RemoteServiceExecutionFailed {
                reason: format!("JSON Parse Error: {}", e),
            }
        })?;
        Ok(Json(json))
    } else {
        Err(aiome_core::error::AiomeError::RemoteServiceError {
            url,
            source: anyhow::anyhow!("Ollama returned error: {}", res.status()),
        }
        .into())
    }
}
