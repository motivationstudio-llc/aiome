/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 */

use axum::{
    extract::{State, Json},
    response::IntoResponse,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use crate::{AppState, auth::Authenticated};
use url::Url;
use std::net::{IpAddr, ToSocketAddrs};
use tracing::{info, error, warn};

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    pub key: String,
    pub value: String,
    pub category: String,
}

#[derive(Debug, Deserialize)]
pub struct TestConnectionRequest {
    pub service: String, // "ollama", "discord", "telegram"
    pub url: String,
    pub token: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub message: String,
}

pub async fn get_settings(
    State(state): State<AppState>,
    _auth: Authenticated,
) -> impl IntoResponse {
    match state.job_queue.fetch_all_settings().await {
        Ok(settings) => {
            // Mask secrets
            let mut masked = settings;
            for s in &mut masked {
                if s.is_secret {
                    s.value = "••••••••".to_string();
                }
            }
            Json(masked).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn update_setting(
    State(state): State<AppState>,
    _auth: Authenticated,
    Json(payload): Json<UpdateSettingsRequest>,
) -> impl IntoResponse {
    // 1. Key whitelist check
    let allowed_keys = [
        "ollama_host", "ollama_model", 
        "discord_chat_channel_id", "discord_command_channel_id", "discord_log_channel_id",
        "telegram_chat_id", "watchtower_enabled", 
        "enforce_guardrail", "log_level", "node_id", "samsara_hub_url"
    ];

    if !allowed_keys.contains(&payload.key.as_str()) {
        warn!("🚨 [Security] Unauthorized settings key attempt: {}", payload.key);
        return (StatusCode::BAD_REQUEST, "Unauthorized setting key").into_response();
    }

    // 2. Category validation
    let allowed_categories = ["llm", "channel", "system", "security"];
    if !allowed_categories.contains(&payload.category.as_str()) {
        return (StatusCode::BAD_REQUEST, "Invalid category").into_response();
    }

    // 3. Value length limit (DoS protection)
    if payload.value.len() > 1024 {
        return (StatusCode::BAD_REQUEST, "Value too long (max 1024 chars)").into_response();
    }

    // 4. Server-side is_secret determination
    let secrets = [
        "ollama_host", "discord_token", "telegram_token", "api_server_secret"
    ];
    let is_secret = secrets.contains(&payload.key.as_str());

    // 5. Audit Logging
    info!("🔧 [Settings] Audit: Key '{}' updated (category: {}, secret: {})", 
        payload.key, payload.category, is_secret);

    match state.job_queue.update_setting(&payload.key, &payload.value, &payload.category, is_secret).await {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => {
            error!("❌ [Settings] Update failed for {}: {}", payload.key, e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
        }
    }
}

pub async fn test_connection(
    _state: State<AppState>,
    _auth: Authenticated,
    Json(payload): Json<TestConnectionRequest>,
) -> impl IntoResponse {
    // SSRF Protection
    if let Err(e) = validate_safe_url(&payload.url) {
        return Json(TestConnectionResponse {
            success: false,
            message: format!("SSRF Blocked: {}", e),
        }).into_response();
    }

    match payload.service.as_str() {
        "ollama" => test_ollama(&payload.url, payload.model.as_deref()).await.into_response(),
        _ => Json(TestConnectionResponse {
            success: false,
            message: format!("Service '{}' testing not implemented yet", payload.service),
        }).into_response(),
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
                return Err(format!("Access to private network address is forbidden: {}", ip));
            }
        }
    }

    Ok(())
}

fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private() || v4.is_link_local() || v4.is_loopback() || v4.is_unspecified() || v4.octets() == [169, 254, 169, 254]
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
        .unwrap();

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
                        let found = models.iter().any(|m| {
                            m.get("name").and_then(|n| n.as_str()) == Some(model_name)
                        });
                        if found {
                            Json(TestConnectionResponse {
                                success: true,
                                message: format!("Ollama connection OK. Model '{}' found.", model_name),
                            })
                        } else {
                            Json(TestConnectionResponse {
                                success: false,
                                message: format!("Ollama connection OK, but model '{}' was not found.", model_name),
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
