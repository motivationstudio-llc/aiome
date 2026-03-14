/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */



use aiome_core::llm_provider::EmbeddingProvider;

use async_trait::async_trait;
use axum::http::header::{
    CACHE_CONTROL, CONTENT_SECURITY_POLICY, STRICT_TRANSPORT_SECURITY, X_CONTENT_TYPE_OPTIONS,
    X_FRAME_OPTIONS,
};
use axum::http::HeaderValue;
use axum::{http::StatusCode, response::IntoResponse, response::Json, routing::get, Router};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;
use tracing::{error, info, warn};
use utoipa::OpenApi;

mod api;
mod auth;
mod docker;
mod error;
mod logging;
mod mcp;
mod routes;
mod skill_handler;
mod stream;

use aiome_core::traits::JobQueue;
use shared::health::HealthMonitor;

#[derive(Clone)]
pub struct AppState {
    pub health_monitor: Arc<Mutex<HealthMonitor>>,
    pub job_queue: Arc<infrastructure::job_queue::SqliteJobQueue>,
    pub wasm_skill_manager: Arc<infrastructure::skills::WasmSkillManager>,
    pub skill_forge: Arc<infrastructure::skills::forge::SkillForge>,
    pub docs_path: String,
    pub llm_semaphore: Arc<tokio::sync::Semaphore>,
    pub forge_semaphore: Arc<tokio::sync::Semaphore>,
    pub mcp_sessions: Arc<
        tokio::sync::RwLock<
            std::collections::HashMap<String, tokio::sync::mpsc::UnboundedSender<String>>,
        >,
    >,
    pub mcp_manager: Arc<mcp::client::McpProcessManager>,
    pub artifact_store: Arc<dyn aiome_core::traits::ArtifactStore>,
    pub event_sender: tokio::sync::broadcast::Sender<shared::watchtower::CoreEvent>,
    pub context_engine: Arc<infrastructure::context_engine::ContextEngine>,
    pub soul_mutator: Arc<infrastructure::soul_mutator::SoulMutator>,
    pub provider: Arc<dyn aiome_core::llm_provider::LlmProvider + Send + Sync>,
    pub autonomous_running: Arc<std::sync::atomic::AtomicBool>,
    pub autonomous_config: Arc<tokio::sync::RwLock<Option<aiome_core::biome::AutonomousConfig>>>,
    pub http_client: reqwest::Client,
    pub docker_failures: Arc<tokio::sync::RwLock<std::collections::HashMap<String, u32>>>,
    pub security_policy: shared::security::SecurityPolicy,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let static_path = "apps/api-server/static";
    let docs_path = "../../docs";

    let health_monitor = shared::health::HealthMonitor::new();
    let health_monitor = Arc::new(Mutex::new(health_monitor));

    let db_url = std::env::var("AIOME_DB_PATH")
        .unwrap_or_else(|_| "sqlite://workspace/aiome.db".to_string());
    if !std::path::Path::new("workspace").exists() {
        std::fs::create_dir_all("workspace").expect("Failed to create workspace");
    }

    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect(&db_url.replace("sqlite://", "sqlite:"))
        .await
        .expect("Failed to connect to SQLite for logging");
    let logger_layer = logging::DbLoggerLayer::new(pool);

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(logger_layer)
        .with(tracing_subscriber::filter::LevelFilter::INFO)
        .init();

    let job_queue = infrastructure::job_queue::SqliteJobQueue::new(&db_url)
        .await
        .expect("Failed to init DB");
    let job_queue = Arc::new(job_queue);

    // Dynamic Provider that reads from DB settings
    #[derive(Debug)]
    struct DynamicLlmProvider {
        jq: Arc<infrastructure::job_queue::SqliteJobQueue>,
        client: reqwest::Client,
        fallback_host: String,
        fallback_model: String,
    }

    #[async_trait]
    impl aiome_core::llm_provider::LlmProvider for DynamicLlmProvider {
        async fn complete(
            &self,
            prompt: &str,
            system: Option<&str>,
        ) -> Result<String, aiome_core::error::AiomeError> {
            let provider_type = self
                .jq
                .get_setting_value("llm_provider")
                .await
                .ok()
                .flatten()
                .unwrap_or_else(|| "ollama".to_string());
            let model_setting = self.jq.get_setting_value("llm_model").await.ok().flatten();
            let model = if let Some(m) = model_setting {
                m
            } else if let Ok(Some(m)) = self.jq.get_setting_value("ollama_model").await {
                m
            } else {
                self.fallback_model.clone()
            };

            match provider_type.as_str() {
                "gemini" => {
                    let api_key = if let Ok(key) = std::env::var("GEMINI_API_KEY") {
                        key
                    } else {
                        self.jq
                            .get_setting_value("llm_api_key")
                            .await
                            .ok()
                            .flatten()
                            .unwrap_or_default()
                    };
                    aiome_core::llm_provider::GeminiProvider::new(
                        self.client.clone(),
                        api_key,
                        model,
                    )
                    .complete(prompt, system)
                    .await
                }
                "openai" => {
                    let api_key = if let Ok(key) = std::env::var("OPENAI_API_KEY") {
                        key
                    } else {
                        self.jq
                            .get_setting_value("llm_api_key")
                            .await
                            .ok()
                            .flatten()
                            .unwrap_or_default()
                    };
                    aiome_core::llm_provider::OpenAiProvider::new(
                        self.client.clone(),
                        api_key,
                        model,
                    )
                    .complete(prompt, system)
                    .await
                }
                "claude" => {
                    let api_key = if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
                        key
                    } else {
                        self.jq
                            .get_setting_value("llm_api_key")
                            .await
                            .ok()
                            .flatten()
                            .unwrap_or_default()
                    };
                    aiome_core::llm_provider::ClaudeProvider::new(
                        self.client.clone(),
                        api_key,
                        model,
                    )
                    .complete(prompt, system)
                    .await
                }
                "lmstudio" => {
                    let host = self
                        .jq
                        .get_setting_value("lm_studio_host")
                        .await
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| "http://127.0.0.1:1234".to_string());
                    aiome_core::llm_provider::LmStudioProvider::new(
                        self.client.clone(),
                        host,
                        model,
                    )
                    .complete(prompt, system)
                    .await
                }
                _ => {
                    let host = self
                        .jq
                        .get_setting_value("ollama_host")
                        .await
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| self.fallback_host.clone());
                    aiome_core::llm_provider::OllamaProvider::new(host, model)
                        .complete(prompt, system)
                        .await
                }
            }
        }
        async fn stream_complete(
            &self,
            prompt: &str,
            system: Option<&str>,
        ) -> Result<
            std::pin::Pin<
                Box<
                    dyn tokio_stream::Stream<Item = Result<String, aiome_core::error::AiomeError>>
                        + Send,
                >,
            >,
            aiome_core::error::AiomeError,
        > {
            let provider_type = self
                .jq
                .get_setting_value("llm_provider")
                .await
                .ok()
                .flatten()
                .unwrap_or_else(|| "ollama".to_string());
            let model_setting = self.jq.get_setting_value("llm_model").await.ok().flatten();
            let model = if let Some(m) = model_setting {
                m
            } else if let Ok(Some(m)) = self.jq.get_setting_value("ollama_model").await {
                m
            } else {
                self.fallback_model.clone()
            };

            match provider_type.as_str() {
                "gemini" => {
                    let api_key = if let Ok(key) = std::env::var("GEMINI_API_KEY") {
                        key
                    } else {
                        self.jq
                            .get_setting_value("llm_api_key")
                            .await
                            .ok()
                            .flatten()
                            .unwrap_or_default()
                    };
                    aiome_core::llm_provider::GeminiProvider::new(
                        self.client.clone(),
                        api_key,
                        model,
                    )
                    .stream_complete(prompt, system)
                    .await
                }
                "openai" => {
                    let api_key = if let Ok(key) = std::env::var("OPENAI_API_KEY") {
                        key
                    } else {
                        self.jq
                            .get_setting_value("llm_api_key")
                            .await
                            .ok()
                            .flatten()
                            .unwrap_or_default()
                    };
                    aiome_core::llm_provider::OpenAiProvider::new(
                        self.client.clone(),
                        api_key,
                        model,
                    )
                    .stream_complete(prompt, system)
                    .await
                }
                "claude" => {
                    let api_key = if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
                        key
                    } else {
                        self.jq
                            .get_setting_value("llm_api_key")
                            .await
                            .ok()
                            .flatten()
                            .unwrap_or_default()
                    };
                    aiome_core::llm_provider::ClaudeProvider::new(
                        self.client.clone(),
                        api_key,
                        model,
                    )
                    .stream_complete(prompt, system)
                    .await
                }
                "lmstudio" => {
                    let host = self
                        .jq
                        .get_setting_value("lm_studio_host")
                        .await
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| "http://127.0.0.1:1234".to_string());
                    aiome_core::llm_provider::LmStudioProvider::new(
                        self.client.clone(),
                        host,
                        model,
                    )
                    .stream_complete(prompt, system)
                    .await
                }
                _ => {
                    let host = self
                        .jq
                        .get_setting_value("ollama_host")
                        .await
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| self.fallback_host.clone());
                    aiome_core::llm_provider::OllamaProvider::new(host, model)
                        .stream_complete(prompt, system)
                        .await
                }
            }
        }
        async fn test_connection(&self) -> Result<(), aiome_core::error::AiomeError> {
            // Delegate to the underlying provider's test_connection
            self.complete("ping", None).await?;
            Ok(())
        }
        fn name(&self) -> &str {
            "DynamicLlm"
        }
    }

    #[async_trait]
    impl aiome_core::llm_provider::EmbeddingProvider for DynamicLlmProvider {
        async fn embed(
            &self,
            text: &str,
            is_query: bool,
        ) -> Result<Vec<f32>, aiome_core::error::AiomeError> {
            let provider_type = self
                .jq
                .get_setting_value("llm_provider")
                .await
                .ok()
                .flatten()
                .unwrap_or_else(|| "ollama".to_string());
            let model_setting = self.jq.get_setting_value("llm_model").await.ok().flatten();
            let model = if let Some(m) = model_setting {
                m
            } else if let Ok(Some(m)) = self.jq.get_setting_value("ollama_model").await {
                m
            } else {
                self.fallback_model.clone()
            };

            match provider_type.as_str() {
                "gemini" => {
                    let api_key = if let Ok(key) = std::env::var("GEMINI_API_KEY") {
                        key
                    } else {
                        self.jq
                            .get_setting_value("llm_api_key")
                            .await
                            .ok()
                            .flatten()
                            .unwrap_or_default()
                    };
                    aiome_core::llm_provider::GeminiProvider::new(
                        self.client.clone(),
                        api_key,
                        model,
                    )
                    .embed(text, is_query)
                    .await
                }
                _ => {
                    let host = self
                        .jq
                        .get_setting_value("ollama_host")
                        .await
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| self.fallback_host.clone());
                    aiome_core::llm_provider::OllamaProvider::new(host, model)
                        .embed(text, is_query)
                        .await
                }
            }
        }
        async fn test_connection(&self) -> Result<(), aiome_core::error::AiomeError> {
            self.embed("ping", false).await?;
            Ok(())
        }
        fn name(&self) -> &str {
            "DynamicEmbedding"
        }
    }

    let fallback_model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen3.5:9b".to_string());
    let fallback_host =
        std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());

    let shared_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap_or_default();

    let provider = Arc::new(DynamicLlmProvider {
        jq: job_queue.clone(),
        client: shared_client.clone(),
        fallback_host,
        fallback_model,
    });

    // === Background LLM Provider (for autonomous tasks) ===
    // Uses a SEPARATE provider (default: Gemini Cloud) to avoid competing with Ollama
    #[derive(Debug)]
    struct BackgroundLlmProvider {
        jq: Arc<infrastructure::job_queue::SqliteJobQueue>,
        client: reqwest::Client,
    }

    #[async_trait]
    impl aiome_core::llm_provider::LlmProvider for BackgroundLlmProvider {
        async fn complete(
            &self,
            prompt: &str,
            system: Option<&str>,
        ) -> Result<String, aiome_core::error::AiomeError> {
            // Priority: DB setting > env var > fallback "ollama" (Pattern B: background uses local LLM)
            let provider_type = self
                .jq
                .get_setting_value("bg_llm_provider")
                .await
                .ok()
                .flatten()
                .or_else(|| std::env::var("BG_LLM_PROVIDER").ok())
                .unwrap_or_else(|| "ollama".to_string());

            let model = self
                .jq
                .get_setting_value("bg_llm_model")
                .await
                .ok()
                .flatten()
                .or_else(|| std::env::var("BG_LLM_MODEL").ok())
                .unwrap_or_else(|| "qwen3.5:9b".to_string());

            let api_key = self
                .jq
                .get_setting_value("bg_llm_api_key")
                .await
                .ok()
                .flatten()
                .or_else(|| std::env::var("GEMINI_API_KEY").ok())
                .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
                .unwrap_or_default();

            match provider_type.as_str() {
                "gemini" => {
                    aiome_core::llm_provider::GeminiProvider::new(
                        self.client.clone(),
                        api_key,
                        model,
                    )
                    .complete(prompt, system)
                    .await
                }
                "openai" => {
                    aiome_core::llm_provider::OpenAiProvider::new(
                        self.client.clone(),
                        api_key,
                        model,
                    )
                    .complete(prompt, system)
                    .await
                }
                "claude" => {
                    aiome_core::llm_provider::ClaudeProvider::new(
                        self.client.clone(),
                        api_key,
                        model,
                    )
                    .complete(prompt, system)
                    .await
                }
                "lmstudio" => {
                    let host = self
                        .jq
                        .get_setting_value("lm_studio_host")
                        .await
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| "http://127.0.0.1:1234".to_string());
                    aiome_core::llm_provider::LmStudioProvider::new(
                        self.client.clone(),
                        host,
                        model,
                    )
                    .complete(prompt, system)
                    .await
                }
                _ => {
                    // Fallback to Ollama (not recommended for background)
                    let host = self
                        .jq
                        .get_setting_value("ollama_host")
                        .await
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());
                    aiome_core::llm_provider::OllamaProvider::new(host, model)
                        .complete(prompt, system)
                        .await
                }
            }
        }

        async fn stream_complete(
            &self,
            prompt: &str,
            system: Option<&str>,
        ) -> Result<
            std::pin::Pin<
                Box<
                    dyn tokio_stream::Stream<Item = Result<String, aiome_core::error::AiomeError>>
                        + Send,
                >,
            >,
            aiome_core::error::AiomeError,
        > {
            // Background tasks don't need streaming. Wrap complete() into a single-item stream.
            let text = self.complete(prompt, system).await?;
            let s = async_stream::stream! { yield Ok(text); };
            Ok(Box::pin(s))
        }

        async fn test_connection(&self) -> Result<(), aiome_core::error::AiomeError> {
            self.complete("ping", None).await?;
            Ok(())
        }
        fn name(&self) -> &str {
            "BackgroundLlm"
        }
    }

    #[async_trait]
    impl aiome_core::llm_provider::EmbeddingProvider for BackgroundLlmProvider {
        async fn embed(
            &self,
            text: &str,
            is_query: bool,
        ) -> Result<Vec<f32>, aiome_core::error::AiomeError> {
            let embed_provider =
                std::env::var("EMBEDDING_PROVIDER").unwrap_or_else(|_| "ruri".to_string());

            match embed_provider.as_str() {
                "ruri" => {
                    // Primary: ruri-v3 local embedding (free, Japanese-optimized)
                    let ruri_url = std::env::var("RURI_EMBED_URL")
                        .unwrap_or_else(|_| "http://localhost:8100".to_string());
                    let ruri = aiome_core::llm_provider::RuriProvider::new(
                        self.client.clone(),
                        ruri_url.clone(),
                    );
                    match ruri.embed(text, is_query).await {
                        Ok(vec) => Ok(vec),
                        Err(e) => {
                            warn!(
                                "⚠️ Ruri embedding failed ({}), falling back to Gemini: {}",
                                ruri_url, e
                            );
                            self.gemini_embed_fallback(text, is_query).await
                        }
                    }
                }
                "gemini" => self.gemini_embed_fallback(text, is_query).await,
                _ => {
                    // Ollama embedding (requires nomic-embed-text or similar)
                    let host = self
                        .jq
                        .get_setting_value("ollama_host")
                        .await
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());
                    let model = self
                        .jq
                        .get_setting_value("bg_llm_model")
                        .await
                        .ok()
                        .flatten()
                        .or_else(|| std::env::var("BG_LLM_MODEL").ok())
                        .unwrap_or_else(|| "qwen3.5:9b".to_string());
                    aiome_core::llm_provider::OllamaProvider::new(host, model)
                        .embed(text, is_query)
                        .await
                }
            }
        }
        async fn test_connection(&self) -> Result<(), aiome_core::error::AiomeError> {
            self.embed("ping", false).await?;
            Ok(())
        }
        fn name(&self) -> &str {
            "BackgroundEmbedding"
        }
    }

    impl BackgroundLlmProvider {
        async fn gemini_embed_fallback(
            &self,
            text: &str,
            is_query: bool,
        ) -> Result<Vec<f32>, aiome_core::error::AiomeError> {
            let mut api_key = self
                .jq
                .get_setting_value("bg_llm_api_key")
                .await
                .ok()
                .flatten()
                .unwrap_or_default();
            if api_key.is_empty() {
                api_key = self
                    .jq
                    .get_setting_value("llm_api_key")
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_default();
            }
            if api_key.is_empty() {
                api_key = std::env::var("GEMINI_API_KEY").unwrap_or_default();
            }
            if api_key.is_empty() {
                return Err(aiome_core::error::AiomeError::Infrastructure {
                    reason: "No embedding provider available: ruri-embed-server not running and no Gemini API key configured".into()
                });
            }
            aiome_core::llm_provider::GeminiProvider::new(
                self.client.clone(),
                api_key,
                "gemini-embedding-001".to_string(),
            )
            .embed(text, is_query)
            .await
        }
    }

    let bg_instance = Arc::new(BackgroundLlmProvider {
        jq: job_queue.clone(),
        client: shared_client.clone(),
    });

    let bg_provider: Arc<dyn aiome_core::llm_provider::LlmProvider> = bg_instance.clone();
    let embed_provider: Arc<dyn aiome_core::llm_provider::EmbeddingProvider> = bg_instance.clone();

    // Wire embedding provider back to job_queue (resolves circular dependency)
    job_queue
        .set_embedding_provider(embed_provider.clone())
        .await;

    let embed_type = std::env::var("EMBEDDING_PROVIDER").unwrap_or_else(|_| "ruri".to_string());
    info!(
        "🧠 [LLM] Front-end: DynamicLlm (DB-configured), Background: {} ({}), Embedding: {}",
        std::env::var("BG_LLM_PROVIDER").unwrap_or_else(|_| "ollama".to_string()),
        std::env::var("BG_LLM_MODEL").unwrap_or_else(|_| "qwen3.5:9b".to_string()),
        embed_type,
    );

    let artifact_store = infrastructure::artifact_store::SqliteArtifactStore::new(
        job_queue.get_pool().clone(),
        std::path::PathBuf::from("workspace/artifacts"),
    )
    .with_embeddings(embed_provider.clone());

    let artifact_store = Arc::new(artifact_store);

    let wasm_skill_manager = Arc::new(
        infrastructure::skills::WasmSkillManager::new("workspace/skills", "workspace")
            .expect("Skills directory not found"),
    );
    let skill_forge = Arc::new(infrastructure::skills::forge::SkillForge::new(
        "workspace/forge",
        "workspace/skills/custom",
    ));

    let llm_semaphore = Arc::new(tokio::sync::Semaphore::new(1)); // Ollama handles 1 request at a time
    let forge_semaphore = Arc::new(tokio::sync::Semaphore::new(1));
    let event_sender = tokio::sync::broadcast::channel::<shared::watchtower::CoreEvent>(100).0;

    skill_forge
        .ensure_forge_workspace()
        .expect("Failed to initialize skill_forge workspace");

    let origins_str = std::env::var("ALLOWED_ORIGINS").unwrap_or_else(|_| 
        "http://127.0.0.1:3015,http://127.0.0.1:3016,http://localhost:1420,http://localhost:1421,http://localhost:5173,http://localhost:3016".to_string()
    );
    let mut all_origins: Vec<String> = origins_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Merge DB-stored origins (requires server restart to take effect)
    if let Ok(Some(db_origins)) = job_queue.get_setting_value("allowed_origins").await {
        for origin in db_origins.split(',') {
            let trimmed = origin.trim().to_string();
            if !trimmed.is_empty() && !all_origins.contains(&trimmed) {
                info!("🌐 [CORS] Adding DB-managed origin: {}", trimmed);
                all_origins.push(trimmed);
            }
        }
    }

    let allowed_origins: Vec<HeaderValue> = all_origins
        .iter()
        .filter_map(|s| s.parse::<HeaderValue>().ok())
        .collect();
    info!("🌐 [CORS] Active origins: {:?}", all_origins);

    info!("🌐 [CORS] Effective Allowed Origins: {:?}", all_origins);
    let cors_layer = CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed_origins))
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ]);
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .redirect(reqwest::redirect::Policy::none()) // C5: Harden SSRF by disabling redirects
        .build()
        .unwrap_or_default();
    let client_bg_clone = http_client.clone();

    let app = build_app(
        AppState {
            health_monitor,
            job_queue: job_queue.clone(),
            wasm_skill_manager,
            skill_forge,
            docs_path: docs_path.to_string(),
            llm_semaphore: llm_semaphore.clone(),
            forge_semaphore: forge_semaphore.clone(),
            mcp_sessions: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            mcp_manager: Arc::new(mcp::client::McpProcessManager::new()),
            artifact_store: artifact_store.clone(),
            event_sender: event_sender.clone(),
            context_engine: Arc::new(infrastructure::context_engine::ContextEngine::new(
                provider.clone(),
                job_queue.clone(),
                llm_semaphore.clone(),
            )),
            soul_mutator: Arc::new(infrastructure::soul_mutator::SoulMutator::new(
                provider.clone(),
                std::path::PathBuf::from("workspace"),
            )),
            provider: provider.clone(),
            autonomous_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            autonomous_config: Arc::new(tokio::sync::RwLock::new(None)),
            http_client: http_client,
            docker_failures: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            security_policy: shared::security::SecurityPolicy::default(),
        },
        cors_layer,
        static_path,
    );

    // Initial Security Check (C1)
    let secret_key = std::env::var("API_SERVER_SECRET").unwrap_or_default();
    if secret_key == "dev_secret" || secret_key.is_empty() {
        if cfg!(debug_assertions) {
            warn!("🚨 [SECURITY CRITICAL] API_SERVER_SECRET is set to fallback value or empty.");
            warn!("🚨 Please set a strong random secret in your .env file immediately.");
        } else {
            error!("🚨 [FATAL SECURITY ERROR] API_SERVER_SECRET IS INSECURE OR MISSING!");
            error!("🚨 Aiome will NOT start in release mode without a strong secret.");
            panic!("Insecure API_SERVER_SECRET in release build");
        }
    }

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3015".to_string())
        .parse()
        .expect("Invalid PORT");
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("🌌 Aiome Management Console listening on {}", addr);

    let token = CancellationToken::new();
    let jq_clone = job_queue.clone();
    let token_bg = token.clone();
    tokio::spawn(async move {
        let token = token_bg;
        let token_ws = token.clone();
        // Initialize LLM for background tasks (using bg_provider to avoid Ollama competition)
        let immune_system =
            infrastructure::immune_system::AdaptiveImmuneSystem::new(bg_provider.clone());

        // Heartbeat Wakeup Setup (Phase 1)
        let wakeup_provider = bg_provider.clone();
        let llm_semaphore = llm_semaphore.clone();
        let event_sender = event_sender.clone();
        let heartbeat_service = infrastructure::heartbeat_wakeup::HeartbeatWakeupService::new(
            wakeup_provider.clone(),
            llm_semaphore.clone(),
        );
        let crystallizer = infrastructure::memory_crystallizer::MemoryCrystallizer::new(
            wakeup_provider.clone(),
            jq_clone.clone(),
            llm_semaphore.clone(),
        );
        let learner =
            infrastructure::user_learner::UserLearner::new(wakeup_provider, llm_semaphore.clone());
        let mut wakeup_counter = 0;

        // 🌐 2. Federation Sync: Connect to Samsara Hub WebSocket for real-time updates
        let hub_ws_url = std::env::var("SAMSARA_HUB_WS")
            .unwrap_or_else(|_| "ws://127.0.0.1:3016/api/v1/federation/ws".to_string());
        let hub_secret = std::env::var("FEDERATION_SECRET").expect("FEDERATION_SECRET must be set");
        let jq_ws = jq_clone.clone();
        let provider_ws = provider.clone();

        tokio::spawn(async move {
            let token = token_ws;
            use aiome_core::contracts::HubMessage;
            use futures_util::{SinkExt, StreamExt};
            use tokio_tungstenite::tungstenite::client::IntoClientRequest;

            let self_node_id = jq_ws.get_node_id().await.unwrap_or_default();
            info!(
                "⚙️ [FederationWorker] Starting with Node ID: {}",
                self_node_id
            );
            let immune_system =
                infrastructure::immune_system::AdaptiveImmuneSystem::new(provider_ws);

            loop {
                if token.is_cancelled() {
                    info!("🛑 [FederationWorker] Shutdown requested. Exiting loop.");
                    break;
                }

                let request_res = hub_ws_url.clone().into_client_request();
                let mut request = match request_res {
                    Ok(req) => req,
                    Err(e) => {
                        error!(
                            "🛑 [FederationWorker] Invalid WS URL: {}. Retrying in 30s...",
                            e
                        );
                        tokio::time::sleep(Duration::from_secs(30)).await;
                        continue;
                    }
                };

                let auth_val = format!("Bearer {}", hub_secret).parse();
                match auth_val {
                    Ok(val) => {
                        request.headers_mut().insert("Authorization", val);
                    }
                    Err(e) => {
                        error!("🛑 [FederationWorker] Failed to parse Authorization header: {}. Retrying in 30s...", e);
                        tokio::time::sleep(Duration::from_secs(30)).await;
                        continue;
                    }
                }

                match tokio_tungstenite::connect_async(request).await {
                    Ok((mut ws_stream, _)) => {
                        info!("🌐 [FederationWorker] Connected to Samsara Hub.");
                        while let Some(msg) = ws_stream.next().await {
                            match msg {
                                Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                                    if let Ok(hub_msg) = serde_json::from_str::<HubMessage>(&text) {
                                        match hub_msg {
                                            HubMessage::NewImmuneRule(rule) => {
                                                // Gap 3 Mitigation: Echo Loop Prevention
                                                if rule.node_id == self_node_id {
                                                    continue;
                                                }
                                                info!("🛡️ [FederationWorker] Received remote rule: {}", rule.pattern);
                                                let _ = jq_ws.store_immune_rule(&rule).await;
                                            }
                                            HubMessage::NewKarma(karma) => {
                                                if karma.node_id == self_node_id {
                                                    continue;
                                                }
                                                info!("🧬 [FederationWorker] Received remote karma: {}", karma.id);
                                                // Normally handled by REST sync, but real-time push is also possible
                                            }
                                            HubMessage::LaggedForceSync { .. } => {
                                                warn!("⚠️ [FederationWorker] Hub reported lag. Forcing full sync in next maintenance cycle...");
                                                // Trigger via system state or a channel if needed, for now just wait for BG worker sync
                                            }
                                            HubMessage::Ping { client_time: _ } => {
                                                let _now_rfc = chrono::Utc::now().to_rfc3339();
                                                let pong = HubMessage::Pong {
                                                    server_time: chrono::Utc::now().to_rfc3339(),
                                                };
                                                if let Ok(text) = serde_json::to_string(&pong) {
                                                    let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(text.into())).await;
                                                }
                                            }
                                            HubMessage::BiomeRelay(msg) => {
                                                if msg.recipient_pubkey != self_node_id {
                                                    continue;
                                                }
                                                info!("📫 [FederationWorker] Incoming Biome Message from {}", msg.sender_pubkey);

                                                // 1. Signature Check
                                                let mut valid = false;
                                                let payload = format!(
                                                    "{}:{}:{}",
                                                    msg.sender_pubkey,
                                                    msg.topic_id,
                                                    msg.lamport_clock
                                                );
                                                if let (Ok(pubkey_bytes), Ok(sig_bytes)) = (
                                                    base64::engine::general_purpose::STANDARD
                                                        .decode(&msg.sender_pubkey),
                                                    base64::engine::general_purpose::STANDARD
                                                        .decode(&msg.signature),
                                                ) {
                                                    let pubkey_arr: [u8; 32] = pubkey_bytes
                                                        .as_slice()
                                                        .try_into()
                                                        .unwrap_or([0; 32]);
                                                    if let (Ok(pubkey), Ok(sig)) = (
                                                        ed25519_dalek::VerifyingKey::from_bytes(
                                                            &pubkey_arr,
                                                        ),
                                                        ed25519_dalek::Signature::from_slice(
                                                            &sig_bytes,
                                                        ),
                                                    ) {
                                                        use ed25519_dalek::Verifier;
                                                        if pubkey
                                                            .verify(payload.as_bytes(), &sig)
                                                            .is_ok()
                                                        {
                                                            valid = true;
                                                        }
                                                    }
                                                }

                                                if !valid {
                                                    warn!("🛡️ [FederationWorker] Invalid Biome Signature from {}", msg.sender_pubkey);
                                                    continue;
                                                }

                                                // 2. Immune system Check (Intent analysis)
                                                if let Ok(Some(rule)) = immune_system
                                                    .verify_intent(&msg.content, jq_ws.as_ref())
                                                    .await
                                                {
                                                    warn!("🛡️ [FederationWorker] Biome Message blocked by Immune System! Pattern: {}", rule.pattern);
                                                    continue;
                                                }

                                                // 3. Store
                                                let _ = sqlx::query("INSERT INTO biome_messages (sender_pubkey, recipient_pubkey, topic_id, content, karma_root_cid, signature, lamport_clock, encryption) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
                                                    .bind(&msg.sender_pubkey).bind(&msg.recipient_pubkey).bind(&msg.topic_id).bind(&msg.content).bind(&msg.karma_root_cid).bind(&msg.signature).bind(msg.lamport_clock as i64).bind(&msg.encryption)
                                                    .execute(jq_ws.get_pool()).await;

                                                let _ = sqlx::query("INSERT INTO biome_peers (pubkey) VALUES (?) ON CONFLICT(pubkey) DO UPDATE SET last_seen_at = datetime('now')")
                                                    .bind(&msg.sender_pubkey).execute(jq_ws.get_pool()).await;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("⚠️ [FederationWorker] WS Stream Error: {:?}", e);
                                    break;
                                }
                                _ => {}
                            }
                        }
                        warn!("🔌 [FederationWorker] WebSocket disconnected. Recalibrating...");
                    }
                    Err(e) => {
                        warn!(
                            "⚠️ [FederationWorker] Connection failed: {:?}. Retrying...",
                            e
                        );
                    }
                }
                tokio::select! {
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(10)) => {},
                    _ = token.cancelled() => {
                        info!("🛑 [FederationWorker] Cancellation received during wait. Exiting.");
                        break;
                    }
                }
            }
        });

        loop {
            if token.is_cancelled() {
                info!("🛑 [BackgroundWorker] Shutdown requested. Cleaning up...");
                break;
            }

            // 🌟 0. Evolution: Sync Samsara Level and handle Behavioral Shift
            let stats = jq_clone.get_agent_stats().await.unwrap_or_default();
            let current_level = stats.level;

            match jq_clone.sync_samsara_level().await {
                Ok(Some(aiome_core::contracts::SamsaraEvent::LevelUp {
                    old_level,
                    new_level,
                })) => {
                    info!(
                        "🌟 [Evolution] Level Up Detected: {} -> {}",
                        old_level, new_level
                    );
                    let mutator = infrastructure::soul_mutator::SoulMutator::new(
                        provider.clone(),
                        std::path::PathBuf::from("workspace"),
                    )
                    .with_prosecutor(provider.clone()); // Self-prosecution for MVP

                    if let Err(e) = mutator
                        .evolve_tactics(jq_clone.as_ref(), old_level, new_level)
                        .await
                    {
                        warn!("⚠️ [Evolution] Behavioral Shift failed: {:?}", e);
                    }
                }
                Ok(None) => {}
                Err(e) => warn!("⚠️ [Evolution] Level sync failed: {:?}", e),
            }

            // 💤 0.5 Contemplation: Dream State (when idle)
            let pending_jobs = jq_clone.get_pending_job_count().await.unwrap_or(0);
            if pending_jobs == 0 {
                let dream_state = infrastructure::dream_state::DreamState::new();
                let search_api_key =
                    std::env::var("SEARCH_API_KEY").unwrap_or_else(|_| "none".to_string());
                let trend_sonar =
                    infrastructure::trend_sonar::ExternalTrendSonar::new(search_api_key);

                if let Err(e) = dream_state
                    .dream(jq_clone.as_ref(), &trend_sonar, current_level)
                    .await
                {
                    warn!("⚠️ [DreamState] Contemplation failed: {:?}", e);
                }
            }

            // 🛡️ 1. Auto-Healing: Analyze threats and generate new immune rules
            // Use try_acquire + short timeout to avoid blocking front-end requests
            if let Ok(_bg_permit) = llm_semaphore.try_acquire() {
                info!(
                    "⚙️ [BackgroundWorker] Starting autonomous threat analysis (Auto-Healing)..."
                );
                match tokio::time::timeout(
                    tokio::time::Duration::from_secs(30),
                    immune_system.analyze_threats(jq_clone.as_ref()),
                )
                .await
                {
                    Ok(Ok(n)) if n > 0 => {
                        info!("🛡️ [BackgroundWorker] {} new immune rules generated.", n)
                    }
                    Ok(Ok(_)) => info!("🛡️ [BackgroundWorker] No new threats identified."),
                    Ok(Err(e)) => warn!("⚠️ [BackgroundWorker] Threat analysis failed: {:?}", e),
                    Err(_) => {
                        warn!("⏭️ [BackgroundWorker] Threat analysis timed out (30s), skipping.")
                    }
                }
            } else {
                info!("⏭️ [BackgroundWorker] LLM busy, skipping threat analysis.");
            }

            // 🧬 1.5 Soul Mutation: Attempt autonomous evolution
            let mutator = infrastructure::soul_mutator::SoulMutator::new(
                bg_provider.clone(),
                std::path::PathBuf::from("workspace"),
            )
            .with_prosecutor(bg_provider.clone());
            if let Ok(_bg_permit) = llm_semaphore.try_acquire() {
                info!("⚙️ [BackgroundWorker] Checking for Soul Mutation (Autonomous Evolution)...");
                match tokio::time::timeout(
                    tokio::time::Duration::from_secs(30),
                    mutator.transmute(jq_clone.as_ref()),
                )
                .await
                {
                    Ok(Ok(true)) => info!("🧬 [BackgroundWorker] Soul mutated successfully."),
                    Ok(Ok(false)) => info!("🧬 [BackgroundWorker] No soul mutation triggered."),
                    Ok(Err(e)) => warn!("⚠️ [BackgroundWorker] Soul mutation failed: {:?}", e),
                    Err(_) => {
                        warn!("⏭️ [BackgroundWorker] Soul mutation timed out (30s), skipping.")
                    }
                }
            } else {
                info!("⏭️ [BackgroundWorker] LLM busy, skipping soul mutation.");
            }

            // 🎭 1.7 Autonomous Expression (Phase 4): Self-Expression based on Karma
            if wakeup_counter % 5 == 0 {
                if let Ok(true) = jq_clone.get_auto_expression_enabled().await {
                    if let Ok(_bg_permit) = llm_semaphore.try_acquire() {
                        info!("⚙️ [BackgroundWorker] Auto-Expression is enabled. Generating...");
                        let karma = jq_clone.fetch_all_karma(5).await.unwrap_or_default();
                        if !karma.is_empty() {
                            let soul_prompt = mutator.get_active_prompt().await.unwrap_or_default();
                            match tokio::time::timeout(tokio::time::Duration::from_secs(30), aiome_core::expression::engine::ExpressionEngine::generate(&karma, &soul_prompt, bg_provider.as_ref())).await {
                                Ok(Ok(expr)) => {
                                    let _ = jq_clone.store_expression(&expr).await;
                                    info!("🎭 [BackgroundWorker] Autonomous Expression generated: {}", expr.emotion);
                                },
                                Ok(Err(e)) => warn!("⚠️ [BackgroundWorker] Expression generation failed: {:?}", e),
                                Err(_) => warn!("⏭️ [BackgroundWorker] Expression generation timed out (30s), skipping."),
                            }
                        }
                    } else {
                        info!("⏭️ [BackgroundWorker] LLM busy, skipping auto-expression.");
                    }
                }
            }

            // 🌐 2. Swarm Sync: Push local data and Sync remote data via REST API
            info!("🌐 [BackgroundWorker] Starting Swarm Sync cycle...");
            let hub_base = std::env::var("SAMSARA_HUB_REST")
                .unwrap_or_else(|_| "http://127.0.0.1:3016".to_string());
            let hub_secret = match std::env::var("FEDERATION_SECRET") {
                Ok(s) => s,
                Err(_) => {
                    error!("🛑 [BackgroundWorker] FEDERATION_SECRET missing. Skipping Swarm Sync.");
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    continue;
                }
            };
            let client = client_bg_clone.clone();

            use aiome_core::contracts::{
                FederationPushRequest, FederationSyncRequest, FederationSyncResponse,
            };

            // 2-A. Push local unfederated data
            if let Ok((karmas, rules)) = jq_clone.fetch_unfederated_data().await {
                let karmas: Vec<aiome_core::contracts::FederatedKarma> = karmas;
                let rules: Vec<aiome_core::contracts::ImmuneRule> = rules;
                if !karmas.is_empty() || !rules.is_empty() {
                    let self_node_id = jq_clone.get_node_id().await.unwrap_or_default();
                    info!(
                        "📤 [BackgroundWorker] Pushing {} Karmas and {} Rules to Hub.",
                        karmas.len(),
                        rules.len()
                    );
                    let push_req = FederationPushRequest {
                        node_id: self_node_id,
                        karmas,
                        rules,
                        arena_matches: vec![],
                    };

                    let res = client
                        .post(format!("{}/api/v1/federation/push", hub_base))
                        .header("Authorization", format!("Bearer {}", hub_secret))
                        .json(&push_req)
                        .send()
                        .await;

                    if let Ok(r) = res {
                        if r.status().is_success() {
                            let k_ids = push_req.karmas.into_iter().map(|k| k.id).collect();
                            let r_ids = push_req.rules.into_iter().map(|r| r.id).collect();
                            let _ = jq_clone.mark_as_federated(k_ids, r_ids).await;
                            info!("✅ [BackgroundWorker] Cloud Push successful.");
                        } else {
                            warn!("⚠️ [BackgroundWorker] Hub rejected Push: {:?}", r.status());
                        }
                    }
                }
            }

            // 2-B. Sync remote approved data with Stateless Pagination (Flaw 2 Defense)
            info!("📥 [BackgroundWorker] Syncing from Hub: {}", hub_base);
            loop {
                let last_sync = jq_clone
                    .get_peer_sync_time("samsara-hub")
                    .await
                    .unwrap_or(None);
                let sync_req = FederationSyncRequest {
                    node_id: jq_clone.get_node_id().await.unwrap_or_default(),
                    since: last_sync,
                    protocol_version: "1.0".to_string(),
                };

                let res = client
                    .post(format!("{}/api/v1/federation/sync", hub_base))
                    .header("Authorization", format!("Bearer {}", hub_secret))
                    .json(&sync_req)
                    .send()
                    .await;

                if let Ok(resp) = res {
                    if resp.status().is_success() {
                        if let Ok(sync_res) = resp.json::<FederationSyncResponse>().await {
                            let karma_len = sync_res.new_karmas.len();
                            let rule_len = sync_res.new_immune_rules.len();
                            let has_more = sync_res.has_more;
                            let server_time = sync_res.server_time.clone();

                            if karma_len > 0 || rule_len > 0 {
                                info!("📥 [BackgroundWorker] Syncing {} new items from Hub (has_more: {}).", karma_len + rule_len, has_more);
                                let _ = jq_clone
                                    .import_federated_data(
                                        sync_res.new_karmas,
                                        sync_res.new_immune_rules,
                                        sync_res.new_arena_matches,
                                    )
                                    .await;
                            }

                            // Update last sync time to the server's processed timestamp for this batch
                            let _ = jq_clone
                                .update_peer_sync_time("samsara-hub", &server_time)
                                .await;

                            if !has_more {
                                break; // Batch complete
                            }
                            // Continue loop for next page
                        } else {
                            break;
                        }
                    } else {
                        warn!(
                            "⚠️ [BackgroundWorker] Hub rejected Sync: {:?}",
                            resp.status()
                        );
                        break;
                    }
                } else {
                    break;
                }
            }

            // 3. Content Publishing: Pick up 'publication' jobs
            if let Ok(Some(job)) = jq_clone.dequeue(&["publication"]).await {
                use infrastructure::publisher::{mock_x::MockXPublisher, PublishPipeline};
                let pipeline = PublishPipeline::new(vec![Box::new(MockXPublisher)]);

                let metadata =
                    serde_json::from_str(job.karma_directives.as_deref().unwrap_or("{}"))
                        .unwrap_or(serde_json::json!({}));
                let platform = metadata["platform"].as_str().unwrap_or("X");

                // For 'publication' jobs, the 'topic' field contains the content string
                let content = job.topic.clone();
                let artifacts_res: Result<Vec<String>, _> =
                    serde_json::from_str(job.output_artifacts.as_deref().unwrap_or("[]"));
                let artifacts: Vec<std::path::PathBuf> = artifacts_res
                    .unwrap_or_default()
                    .into_iter()
                    .map(std::path::PathBuf::from)
                    .collect();

                match pipeline
                    .run_job(platform, &content, &artifacts, &metadata)
                    .await
                {
                    Ok(cid) => {
                        let _ = jq_clone.complete_job(&job.id, None).await;
                        let _ = jq_clone.link_sns_data(&job.id, platform, &cid).await;
                        info!(
                            "✅ [BackgroundWorker] Publication successful (ID: {}).",
                            cid
                        );
                    }
                    Err(e) => {
                        let _ = jq_clone.fail_job(&job.id, &e.to_string()).await;
                        warn!("⚠️ [BackgroundWorker] Publication failed: {:?}", e);
                    }
                }
            }

            // 5. Memory Evolution: Procedural Forgetting Sweep
            if let Ok(archived) = jq_clone.karma_decay_sweep().await {
                if archived > 0 {
                    info!("♻️ [BackgroundWorker] Memory Evolution: Archived {} faint memories via decay sweep.", archived);
                }
            }

            // 4. Storage GC: Maintain clean environment (Threshold: 10GB)
            if let Ok(purged) = jq_clone.storage_gc(10.0).await {
                if purged > 0 {
                    info!(
                        "♻️ [BackgroundWorker] Storage GC: Purged {} old artifacts.",
                        purged
                    );
                }
            }

            // 6. Heartbeat Wakeup Ping (Phase 1) - Every 30 maintenance cycles (~30 mins)
            if wakeup_counter % 30 == 0 {
                if let Some(msg) = heartbeat_service.run_wakeup_ping().await {
                    let _ = event_sender.send(shared::watchtower::CoreEvent::ProactiveTalk {
                        message: msg,
                        channel_id: 0,
                    });
                    info!("💓 [BackgroundWorker] Heartbeat: Proactive talk dispatched.");
                }
            }
            wakeup_counter = (wakeup_counter + 1) % 1440; // Prevent overflow, reset dailyish

            // 7. Memory Crystallization (Phase 2) - Daily maintenance
            if wakeup_counter == 0 {
                info!("💎 [BackgroundWorker] Memory Evolution: Starting Crystallization cycle...");
                let _ = crystallizer.run_distillation_cycle().await;
            }

            // 8. User Learning (Phase 2) - Hourly preference updates
            if wakeup_counter % 60 == 0 {
                if let Ok(channels) = jq_clone.fetch_undistilled_chats_by_channel().await {
                    for (channel_id, messages) in channels {
                        let summary = messages
                            .iter()
                            .map(|(_, role, content)| format!("{}: {}", role, content))
                            .collect::<Vec<_>>()
                            .join("\n");
                        if learner.learn_from_session(&summary).await.unwrap_or(false) {
                            let last_id = messages.last().map(|(id, ..)| *id).unwrap_or(0);
                            let _ = jq_clone.mark_chats_as_distilled(&channel_id, last_id).await;
                        }
                    }
                }
            }

            // 9. Knowledge Indexing (Phase 21-B) - Refresh project knowledge every 12 cycles (~1 hour)
            // Trigger on first cycle (counter=1) for immediate indexing
            if wakeup_counter == 1 || wakeup_counter % 12 == 0 {
                let ws_root =
                    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                let indexer = infrastructure::knowledge_indexer::ProjectKnowledgeIndexer::new(
                    artifact_store.clone(),
                    jq_clone.get_pool().clone(),
                    ws_root,
                );
                let _ = indexer.run_indexing().await;
            }

            // 10. SQLite Global Backup (Tier 5: Architecture) - Every 2 hours (24 cycles)
            if wakeup_counter % 24 == 0 {
                info!("💾 [BackgroundWorker] Starting SQLite periodic backup...");
                let backup_dir = std::path::Path::new("workspace/backups");
                if !backup_dir.exists() {
                    let _ = std::fs::create_dir_all(backup_dir);
                }
                let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
                let backup_path = backup_dir.join(format!("aiome_{}.db", timestamp));

                // Using VACUUM INTO for online backup (safe even if the file is being written to)
                let pool = jq_clone.get_pool();
                // Ensure we use the absolute path for SQLite
                if let Ok(abs_backup_path) = std::fs::canonicalize(backup_dir)
                    .map(|p| p.join(format!("aiome_{}.db", timestamp)))
                {
                    let query = format!(
                        "VACUUM INTO '{}'",
                        abs_backup_path.to_str().unwrap_or_default()
                    );
                    match sqlx::query(&query).execute(pool).await {
                        Ok(_) => info!(
                            "💾 [BackgroundWorker] Backup successful: {:?}",
                            abs_backup_path
                        ),
                        Err(e) => warn!("⚠️ [BackgroundWorker] Backup failed: {:?}", e),
                    }
                } else {
                    // Fallback to relative if canonicalize fails (e.g. dir just created)
                    let query =
                        format!("VACUUM INTO '{}'", backup_path.to_str().unwrap_or_default());
                    match sqlx::query(&query).execute(pool).await {
                        Ok(_) => info!(
                            "💾 [BackgroundWorker] Backup successful (relative): {:?}",
                            backup_path
                        ),
                        Err(e) => warn!("⚠️ [BackgroundWorker] Backup failed: {:?}", e),
                    }
                }

                // Cleanup old backups (keep last 5)
                if let Ok(entries) = std::fs::read_dir(backup_dir) {
                    let mut paths: Vec<_> = entries.flatten().map(|e| e.path()).collect();
                    paths.sort_by(|a, b| {
                        let ma = a
                            .metadata()
                            .and_then(|m| m.modified())
                            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                        let mb = b
                            .metadata()
                            .and_then(|m| m.modified())
                            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                        mb.cmp(&ma)
                    });

                    if paths.len() > 5 {
                        for old_path in paths.iter().skip(5) {
                            let _ = std::fs::remove_file(old_path);
                        }
                    }
                }
            }

            // Sleep for 5 minutes before next maintenance cycle (Pattern B: longer interval for Ollama background)
            tokio::select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(300)) => {},
                _ = token.cancelled() => {
                    info!("🛑 [BackgroundWorker] Cancellation received. Exiting.");
                    break;
                }
            }
        }
    });

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to port 3015");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(token))
        .await
        .expect("Server error");
}

async fn shutdown_signal(token: CancellationToken) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("🔴 [api-server] Received Ctrl+C signal. Initiating graceful shutdown...");
        },
        _ = terminate => {
            info!("🔴 [api-server] Received Terminate signal. Initiating graceful shutdown...");
        },
    }

    token.cancel();

    // Give background workers some time to cleanup
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    info!("👋 [api-server] Graceful shutdown complete.");
}

pub fn build_app(state: AppState, cors_layer: CorsLayer, static_path: &str) -> Router {
    Router::new()
        // --- Protected Routes (Require Authentication) ---
        .route("/api/wiki", get(routes::general::list_wiki_files))
        .route("/api/wiki/:filename", get(routes::general::get_wiki_content))
        .route("/api/synergy/karma", get(routes::karma::get_karma_stream))
        .route("/api/synergy/graph", get(routes::karma::synergy_graph_handler))
        .route("/api/synergy/test/failure", axum::routing::post(routes::karma::trigger_failure_demo))
        .route("/api/synergy/test/security", axum::routing::post(routes::karma::trigger_security_demo))
        .route("/api/synergy/test/federation", axum::routing::post(routes::karma::trigger_federation_demo))
        .route("/api/synergy/rules", get(routes::karma::get_immune_rules_handler).post(routes::karma::add_immune_rule_handler).put(routes::karma::add_immune_rule_handler))
        .route("/api/synergy/rules/:id", axum::routing::delete(routes::karma::delete_immune_rule_handler))
        .route("/api/artifacts", get(routes::artifacts::list_artifacts_handler))
        .route("/api/artifacts/:id", get(routes::artifacts::get_artifact_handler).delete(routes::artifacts::delete_artifact_handler))
        .route("/api/artifacts/:id/edges", get(routes::artifacts::get_artifact_edges_handler))
        .route("/api/artifacts/:id/files/:filename", get(routes::artifacts::download_artifact_file_handler))
        .route("/api/agent/chat", axum::routing::post(routes::agent::trigger_agent_chat))
        .route("/api/agent/chat/stream", axum::routing::post(stream::trigger_agent_chat_stream))
        .route("/api/agent/feedback", axum::routing::post(routes::agent::handle_karma_feedback))
        .route("/api/system/evolution", get(routes::karma::get_evolution_history_handler))
        .route("/api/v1/settings", get(routes::settings::get_settings).put(routes::settings::update_setting))
        .route("/api/v1/settings/test", axum::routing::post(routes::settings::test_connection))
        .route("/api/v1/ollama/models", get(routes::settings::get_ollama_models))
        .route("/api/v1/logs", get(routes::general::get_logs))
        .route("/api/biome/status", get(routes::biome::biome_status))
        .route("/api/biome/topics", get(routes::biome::list_topics).post(routes::biome::create_topic))
        .route("/api/biome/list", get(routes::biome::list_messages))
        .route("/api/biome/send", axum::routing::post(routes::biome::send_message))
        .route("/api/biome/autonomous/start", axum::routing::post(routes::biome::autonomous_start))
        .route("/api/biome/autonomous/stop", axum::routing::post(routes::biome::autonomous_stop))
        .route("/api/biome/autonomous/status", get(routes::biome::autonomous_status))
        .route("/api/expression/status", get(routes::expression::expression_status))
        .route("/api/expression/generate", axum::routing::post(routes::expression::generate_expression))
        .route("/api/expression/list", get(routes::expression::list_expressions))
        .route("/api/expression/auto", axum::routing::post(routes::expression::toggle_auto_expression))
        .route("/api/skills", get(routes::skill::list_skills))
        .route("/api/skills/import", axum::routing::post(routes::skill::import_skill))
        .route("/api/skills/mcp/spawn", axum::routing::post(routes::skill::spawn_mcp_server))
        .route("/api/health", get(routes::general::get_health_status))
        .nest("/api/v1/mcp", mcp::router())
        .route_layer(axum::middleware::from_extractor::<auth::Authenticated>())

        // --- Public Routes (Internal Monitoring / SSE / WS) ---
        .merge(utoipa_swagger_ui::SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api::ApiDoc::openapi()))
        .route("/api/system/vitality", get(stream::trigger_system_vitality_stream))
        .route("/api/v1/watchtower/ws", get(routes::watchtower::ws_handler))

        .with_state(state) // state
        .fallback_service(ServeDir::new(static_path).append_index_html_on_directories(true))
        // --- Layer 3: Security Headers (Defense in Depth) ---
        .layer(SetResponseHeaderLayer::if_not_present(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")))
        .layer(SetResponseHeaderLayer::if_not_present(X_FRAME_OPTIONS, HeaderValue::from_static("DENY")))
        .layer(SetResponseHeaderLayer::if_not_present(STRICT_TRANSPORT_SECURITY, HeaderValue::from_static("max-age=31536000; includeSubDomains")))
        .layer(SetResponseHeaderLayer::if_not_present(CONTENT_SECURITY_POLICY, HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; connect-src 'self' ws: wss: http: https:; object-src 'none'; base-uri 'self';")))
        // --- Layer 2: Dynamic CORS (Whitelisting) ---
        // Sources: 1) ALLOWED_ORIGINS env var (defaults)  2) DB system_settings (dynamic)
        .layer(cors_layer)
        // --- Layer 1: Global Rate Limiting & DoS Protection ---
        .layer(
            tower::ServiceBuilder::new()
                .layer(axum::error_handling::HandleErrorLayer::new(|err: tower::BoxError| async move {
                    (StatusCode::INTERNAL_SERVER_ERROR, format!("Security Layer Error: {}", err))
                }))
                .buffer(1024)
                .rate_limit(50, std::time::Duration::from_secs(1)) // Spike protection
                .into_inner()
        )
}

#[cfg(test)]
mod api_integration_tests;
