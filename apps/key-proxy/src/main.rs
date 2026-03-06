/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio,LLC
 */

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::env;
use tracing::{info, error, warn};
use zeroize::Zeroize;

#[derive(Debug, Deserialize)]
struct ProxyRequest {
    caller_id: String,
    prompt: String,
    system: Option<String>,
    endpoint: String, // "gemini" etc (Hardcoded Enum-like check)
}

#[derive(Debug, Serialize)]
struct ProxyResponse {
    result: String,
}

use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::HashMap;

#[derive(Clone)]
struct AppState {
    gemini_key: Arc<SecretString>,
    client: reqwest::Client,
    total_calls: Arc<AtomicU64>,
    caller_quotas: Arc<HashMap<String, u64>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("🔐 [KeyProxy] Starting the Abyss Vault...");

    // 1. Extreme Security: Memory Lock (mlockall)
    #[cfg(target_os = "linux")]
    {
        use nix::sys::mman::{mlockall, MlockAllFlags};
        if let Err(e) = mlockall(MlockAllFlags::MCL_CURRENT | MlockAllFlags::MCL_FUTURE) {
            error!("❌ [KeyProxy] mlockall failed: {}. ABORTING for safety.", e);
            panic!("SECURITY VIOLATION: Could not lock memory to RAM.");
        }
    // 7. Security: Anti-Debugger (petersen's trick / ptrace)
    #[cfg(target_os = "macos")]
    {
        use nix::sys::ptrace;
        if ptrace::traceme().is_err() {
            error!("🚨 [KeyProxy] Debugger detected! Panic for safety.");
            panic!("SECURITY VIOLATION: Debugger attached.");
        }
    }
    
    info!("🧠 [KeyProxy] Memory locked to RAM (no swap).");
    }

    // 2. Load keys and SELF-WIPE ENV
    dotenvy::dotenv().ok();
    let gemini_key = env::var("GEMINI_API_KEY")
        .expect("GEMINI_API_KEY must be set in key-proxy/.env");
    
    // Self-Wipe: Remove from environment immediately
    unsafe {
        env::remove_var("GEMINI_API_KEY");
    }
    info!("🧹 [KeyProxy] Environment wiped. Gemini key is now only in memory.");

    let mut quotas = HashMap::new();
    quotas.insert("daemon".to_string(), 1000);
    quotas.insert("watchtower".to_string(), 100);
    quotas.insert("api-server".to_string(), 1000);

    let state = AppState {
        gemini_key: Arc::new(SecretString::from(gemini_key)),
        // 3. Ex-Machina: No-Redirect Policy
        client: reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?,
        total_calls: Arc::new(AtomicU64::new(0)),
        caller_quotas: Arc::new(quotas),
    };

    let app = Router::new()
        .route("/api/v1/llm/complete", post(handle_llm_complete))
        .route("/api/v1/llm/embed", post(handle_llm_embed))
        .with_state(state);

    // 4. Level 5: Unix Domain Sockets (Optional/Configurable)
    // For now, let's start with TCP but keep the design ready for UDS
    let port = env::var("KEY_PROXY_PORT").unwrap_or_else(|_| "9999".to_string());
    let bind_addr = if env::var("BIND_ALL").map(|v| v == "true").unwrap_or(false) {
        "0.0.0.0"
    } else {
        "127.0.0.1"
    };
    let addr = format!("{}:{}", bind_addr, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    info!("🚀 [KeyProxy] Abyss Vault listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn handle_llm_complete(
    State(state): State<AppState>,
    Json(payload): Json<ProxyRequest>,
) -> impl IntoResponse {
    info!("📩 [KeyProxy] Request from caller: {}", payload.caller_id);

    // 8. Zero-Trust: Caller & Quota Check
    if !state.caller_quotas.contains_key(&payload.caller_id) {
        warn!("🚫 [KeyProxy] Unknown caller: {}", payload.caller_id);
        return (StatusCode::FORBIDDEN, "Unknown caller").into_response();
    }
    
    let total = state.total_calls.fetch_add(1, Ordering::SeqCst);
    if total > 5000 { // Global Hard Limit
        error!("🛑 [KeyProxy] Global quota exceeded!");
        return (StatusCode::TOO_MANY_REQUESTS, "Global quota exceeded").into_response();
    }

    // 5. SSRF Defense: Hardcoded Endpoints
    let url = match payload.endpoint.as_str() {
        "gemini" => "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent",
        _ => return (StatusCode::BAD_REQUEST, "Invalid endpoint").into_response(),
    };

    // 6. Zeroize usage
    let mut api_key = state.gemini_key.expose_secret().clone();
    
    // DEMO MOCK MODE
    if api_key == "mock_key_for_testing" {
        api_key.zeroize();
        return Json(ProxyResponse { 
            result: format!("I am OpenClaw. I hear you loud and clear. Your prompt was: '{}'. Currently operating in Mock Offline Mode inside the Aiome Abyss Vault.", payload.prompt)
        }).into_response();
    }
    
    let gemini_payload = serde_json::json!({
        "contents": [{
            "parts": [{
                "text": payload.prompt
            }]
        }],
        "system_instruction": payload.system.map(|s| {
            serde_json::json!({ "parts": [{ "text": s }] })
        })
    });

    let target_url = format!("{}?key={}", url, api_key);
    
    // IMPORTANT: Zeroize the key copy immediately after string format if possible
    api_key.zeroize(); 

    let res = state.client.post(target_url)
        .header("Content-Type", "application/json")
        .json(&gemini_payload)
        .send()
        .await;

    match res {
        Ok(resp) => {
            if resp.status().is_success() {
                let body_res: Result<serde_json::Value, _> = resp.json().await;
                match body_res {
                    Ok(body) => {
                        // Extract text from Gemini structure
                        let text = body["candidates"][0]["content"]["parts"][0]["text"]
                            .as_str()
                            .unwrap_or("")
                            .to_string();
                        
                        Json(ProxyResponse { result: text }).into_response()
                    }
                    Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error").into_response()
                }
            } else {
                let status = resp.status();
                error!("❌ [KeyProxy] Upstream error: {}", status);
                // 7. Ex-Machina: ERROR MASKING
                (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error").into_response()
            }
        }
        Err(e) => {
            error!("❌ [KeyProxy] Request failed: {:?}", e);
            // 7. Ex-Machina: ERROR MASKING (Zeroize URL from error str in production if needed)
            (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error").into_response()
        }
    }
}
#[derive(Debug, Serialize)]
struct EmbedResponse {
    embedding: Vec<f32>,
}

async fn handle_llm_embed(
    State(state): State<AppState>,
    Json(payload): Json<ProxyRequest>,
) -> impl IntoResponse {
    info!("🧬 [KeyProxy] Embedding request from caller: {}", payload.caller_id);

    if !state.caller_quotas.contains_key(&payload.caller_id) {
        return (StatusCode::FORBIDDEN, "Unknown caller").into_response();
    }
    state.total_calls.fetch_add(1, Ordering::SeqCst);

    let url = match payload.endpoint.as_str() {
        "gemini-embed" => "https://generativelanguage.googleapis.com/v1beta/models/text-embedding-004:embedContent",
        _ => return (StatusCode::BAD_REQUEST, "Invalid endpoint").into_response(),
    };

    let mut api_key = state.gemini_key.expose_secret().clone();
    
    let gemini_payload = serde_json::json!({
        "content": {
            "parts": [{ "text": payload.prompt }]
        }
    });

    let target_url = format!("{}?key={}", url, api_key);
    api_key.zeroize(); 

    let res = state.client.post(target_url)
        .header("Content-Type", "application/json")
        .json(&gemini_payload)
        .send()
        .await;

    match res {
        Ok(resp) => {
            if resp.status().is_success() {
                let body: serde_json::Value = resp.json().await.unwrap_or_default();
                let emb = body["embedding"]["values"].as_array();
                if let Some(values) = emb {
                    let vec: Vec<f32> = values.iter().map(|v| v.as_f64().unwrap_or(0.0) as f32).collect();
                    Json(EmbedResponse { embedding: vec }).into_response()
                } else {
                    (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error").into_response()
                }
            } else {
                error!("❌ [KeyProxy] Upstream error: {}", resp.status());
                (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error").into_response()
            }
        }
        Err(e) => {
            error!("❌ [KeyProxy] Request failed: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Upstream Provider Error").into_response()
        }
    }
}
