use axum::{
    extract::{Path, Query, State, Json},
    response::{IntoResponse, Response},
    http::{StatusCode, header},
};
use serde::{Deserialize, Serialize};
use crate::AppState;
use aiome_core::traits::{ArtifactStore, ArtifactCategory, ArtifactMeta};
use std::sync::Arc;
use bastion::fs_guard::Jail;

#[derive(Deserialize)]
pub struct ListArtifactsParams {
    pub category: Option<String>,
    pub q: Option<String>,
    pub limit: Option<i64>,
}

pub async fn list_artifacts_handler(
    State(state): State<AppState>,
    Query(params): Query<ListArtifactsParams>,
) -> impl IntoResponse {
    let category = params.category.and_then(|c| {
        match c.to_lowercase().as_str() {
            "report" => Some(ArtifactCategory::Report),
            "code" => Some(ArtifactCategory::Code),
            "image" => Some(ArtifactCategory::Image),
            "audio" => Some(ArtifactCategory::Audio),
            "expression" => Some(ArtifactCategory::Expression),
            "data" => Some(ArtifactCategory::Data),
            _ => None,
        }
    });

    let limit = params.limit.unwrap_or(50);
    
    let result = if let Some(query) = params.q {
        state.artifact_store.search_artifacts_semantic(&query, category, limit).await
    } else {
        state.artifact_store.list_artifacts(category, limit).await
    };

    match result {
        Ok(artifacts) => (StatusCode::OK, Json(artifacts)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

pub async fn get_artifact_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.artifact_store.fetch_artifact(&id).await {
        Ok(Some(artifact)) => (StatusCode::OK, Json(artifact)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Artifact not found"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

pub async fn download_artifact_file_handler(
    State(state): State<AppState>,
    Path((id, filename)): Path<(String, String)>,
) -> impl IntoResponse {
    // SEC-3: Santize filename input to prevent path traversal at route level
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return (StatusCode::BAD_REQUEST, "Invalid filename").into_response();
    }

    // 1. Create a transient Jail for serving the file (read-only intent)
    let jail = match Jail::new("workspace") {
        Ok(j) => j,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Security error: {}", e)).into_response(),
    };

    match state.artifact_store.read_artifact_file(&id, &filename, &jail).await {
        Ok(content) => {
            let mime_type = mime_guess::from_path(&filename).first_or_octet_stream().to_string();
            Response::builder()
                .header(header::CONTENT_TYPE, mime_type)
                .header(header::CONTENT_DISPOSITION, format!("inline; filename=\"{}\"", filename))
                .body(axum::body::Body::from(content))
                .unwrap_or_else(|e| {
                    (StatusCode::INTERNAL_SERVER_ERROR, format!("Response build error: {}", e)).into_response()
                })
                .into_response()
        },
        Err(aiome_core::error::AiomeError::ArtifactNotFound { .. }) => {
            (StatusCode::NOT_FOUND, "File not found").into_response()
        },
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Error reading file: {}", e)).into_response()
        }
    }
}

pub async fn delete_artifact_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let jail = match Jail::new("workspace") {
        Ok(j) => j,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };

    match state.artifact_store.delete_artifact(&id, &jail).await {
        Ok(_) => (StatusCode::NO_CONTENT, "").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}
pub async fn get_artifact_edges_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.artifact_store.get_artifact_edges(&id).await {
        Ok(edges) => (StatusCode::OK, Json(edges)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}
