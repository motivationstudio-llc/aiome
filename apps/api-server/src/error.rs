/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1 (BSL 1.1).
 * Change Date: 2030-01-01
 * Change License: Apache License 2.0
 */

use aiome_core::error::AiomeError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

pub struct AppError(pub AiomeError);

impl From<AiomeError> for AppError {
    fn from(err: AiomeError) -> Self {
        Self(err)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self(AiomeError::OsError { source: err })
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for AppError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self(AiomeError::OsError {
            source: anyhow::anyhow!("{}", err),
        })
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self.0 {
            AiomeError::PromptBlocked { reason } => (StatusCode::FORBIDDEN, reason.clone()),
            AiomeError::ArtifactNotFound { path } => (
                StatusCode::NOT_FOUND,
                format!("Artifact not found: {}", path),
            ),
            AiomeError::SecurityViolation { reason } => (
                StatusCode::FORBIDDEN,
                format!("Security violation: {}", reason),
            ),
            AiomeError::BudgetExhausted(e) => (
                StatusCode::TOO_MANY_REQUESTS,
                format!("Budget exhausted: {}", e),
            ),
            AiomeError::RemoteServiceTimeout { timeout_secs } => (
                StatusCode::GATEWAY_TIMEOUT,
                format!("Remote service timeout after {}s", timeout_secs),
            ),
            AiomeError::StorageFull { threshold } => (
                StatusCode::INSUFFICIENT_STORAGE,
                format!("Storage is full (leveled at {}%)", threshold),
            ),
            AiomeError::ContextFetch { source }
            | AiomeError::LlmResponse { source }
            | AiomeError::OsError { source } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal error: {}", source),
            ),
            AiomeError::ConfigLoad { source } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Configuration error: {}", source),
            ),
            AiomeError::Infrastructure { reason } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Infrastructure error: {}", reason),
            ),
            AiomeError::RemoteServiceError { url, source } => (
                StatusCode::BAD_GATEWAY,
                format!("Remote service error ({}): {}", url, source),
            ),
            AiomeError::RemoteServiceExecutionFailed { reason } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Execution failed: {}", reason),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("An unexpected error occurred: {}", self.0),
            ),
        };

        let body = Json(json!({
            "error": error_message,
            "code": format!("{:?}", self.0).split('(').next().unwrap_or("Unknown").trim(),
        }));

        (status, body).into_response()
    }
}
