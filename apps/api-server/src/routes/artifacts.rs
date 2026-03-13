use crate::error::AppError;
use crate::AppState;
use aiome_core::traits::{ArtifactCategory, ArtifactMeta, ArtifactStore};
use axum::{
    extract::{Json, Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use bastion::fs_guard::Jail;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ListArtifactsParams {
    pub category: Option<String>,
    pub q: Option<String>,
    pub limit: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/artifacts",
    params(
        ("category" = Option<String>, Query, description = "Filter by category"),
        ("q" = Option<String>, Query, description = "Semantic search query"),
        ("limit" = Option<i64>, Query, description = "Limit results")
    ),
    responses(
        (status = 200, description = "List of artifacts", body = [serde_json::Value])
    )
)]
pub async fn list_artifacts_handler(
    State(state): State<AppState>,
    Query(params): Query<ListArtifactsParams>,
) -> Result<Json<Vec<ArtifactMeta>>, AppError> {
    let category = params
        .category
        .and_then(|c| match c.to_lowercase().as_str() {
            "report" => Some(ArtifactCategory::Report),
            "code" => Some(ArtifactCategory::Code),
            "image" => Some(ArtifactCategory::Image),
            "audio" => Some(ArtifactCategory::Audio),
            "expression" => Some(ArtifactCategory::Expression),
            "data" => Some(ArtifactCategory::Data),
            "knowledge" => Some(ArtifactCategory::Knowledge),
            _ => None,
        });

    let limit = params.limit.unwrap_or(50);

    let artifacts = if let Some(query) = params.q {
        state
            .artifact_store
            .search_artifacts_semantic(&query, category, limit)
            .await?
    } else {
        state.artifact_store.list_artifacts(category, limit).await?
    };

    Ok(Json(artifacts))
}

#[utoipa::path(
    get,
    path = "/api/v1/artifacts/{id}",
    params(
        ("id" = String, Path, description = "Artifact ID")
    ),
    responses(
        (status = 200, description = "Artifact metadata", body = serde_json::Value),
        (status = 404, description = "Artifact not found")
    )
)]
pub async fn get_artifact_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let artifact = state.artifact_store.fetch_artifact(&id).await?;
    match artifact {
        Some(a) => Ok(Json(serde_json::json!(a))),
        None => Err(aiome_core::error::AiomeError::ArtifactNotFound { path: id }.into()),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/artifacts/{id}/files/{filename}",
    params(
        ("id" = String, Path, description = "Artifact ID"),
        ("filename" = String, Path, description = "File name")
    ),
    responses(
        (status = 200, description = "File content stream"),
        (status = 400, description = "Invalid filename"),
        (status = 404, description = "File not found")
    )
)]
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
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Security error: {}", e),
            )
                .into_response()
        }
    };

    match state
        .artifact_store
        .read_artifact_file(&id, &filename, &jail)
        .await
    {
        Ok(content) => {
            let mime_type = mime_guess::from_path(&filename)
                .first_or_octet_stream()
                .to_string();
            Response::builder()
                .header(header::CONTENT_TYPE, mime_type)
                .header(
                    header::CONTENT_DISPOSITION,
                    format!("inline; filename=\"{}\"", filename),
                )
                .body(axum::body::Body::from(content))
                .unwrap_or_else(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Response build error: {}", e),
                    )
                        .into_response()
                })
                .into_response()
        }
        Err(aiome_core::error::AiomeError::ArtifactNotFound { .. }) => {
            (StatusCode::NOT_FOUND, "File not found").into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error reading file: {}", e),
        )
            .into_response(),
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/artifacts/{id}",
    params(
        ("id" = String, Path, description = "Artifact ID")
    ),
    responses(
        (status = 204, description = "Artifact deleted"),
        (status = 500, description = "Internal error")
    )
)]
pub async fn delete_artifact_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let jail = Jail::new("workspace")
        .map_err(|e| aiome_core::error::AiomeError::OsError { source: e.into() })?;

    state.artifact_store.delete_artifact(&id, &jail).await?;
    Ok(StatusCode::NO_CONTENT)
}
#[utoipa::path(
    get,
    path = "/api/v1/artifacts/{id}/edges",
    params(
        ("id" = String, Path, description = "Artifact ID")
    ),
    responses(
        (status = 200, description = "Artifact edges (graph)", body = [serde_json::Value])
    )
)]
pub async fn get_artifact_edges_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<aiome_core::traits::ArtifactEdge>>, AppError> {
    let edges = state.artifact_store.get_artifact_edges(&id).await?;
    Ok(Json(edges))
}
