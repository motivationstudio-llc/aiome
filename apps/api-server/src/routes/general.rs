use axum::{
    routing::get,
    response::Json,
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    extract::State,
};
use std::fs;
use crate::AppState;
use crate::ResourceStatus;
use aiome_core::traits::JobQueue;

pub async fn list_wiki_files(
    State(state): State<AppState>
) -> Json<Vec<String>> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(&state.docs_path) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".md") {
                    files.push(name.to_string());
                }
            }
        }
    }
    files.sort();
    Json(files)
}

pub async fn get_wiki_content(
    State(state): State<AppState>,
    Path(filename): Path<String>
) -> impl IntoResponse {
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return (StatusCode::BAD_REQUEST, "Invalid filename").into_response();
    }

    let path = std::path::PathBuf::from(&state.docs_path).join(filename);
    match fs::read_to_string(path) {
        Ok(content) => content.into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Wiki not found").into_response(),
    }
}

pub async fn get_mock_clouddoc_page(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>
) -> impl IntoResponse {
    let slug = params.get("slug").map(|s| s.as_str()).unwrap_or("philosophy");
    match slug {
        "api-usage" => "# API Usage\nAiome provides a secure, low-latency API proxy.",
        _ => "# Vision & Philosophy\nAiome OS: The Mathematical Sovereignty of Autonomous Agents.",
    }.into_response()
}

pub async fn get_health_status(
    State(state): State<AppState>,
) -> Json<ResourceStatus> {
    let mut monitor = state.health_monitor.lock().await;
    let mut status = monitor.check();
    
    // Fetch real agent stats
    if let Ok(stats) = state.job_queue.get_agent_stats().await {
        status.level = stats.level;
        status.exp = stats.exp;
        status.resonance = stats.resonance;
        status.creativity = stats.creativity;
        status.fatigue = stats.fatigue;
    }
    
    Json(status)
}
