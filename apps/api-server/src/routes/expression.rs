use axum::{
    response::Json,
    extract::State,
};
use crate::AppState;
use aiome_core::traits::JobQueue;

pub async fn expression_status(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let pending_count = state.job_queue.get_pending_job_count().await.unwrap_or(0);
    let recent_karma = state.job_queue.fetch_all_karma(1).await.unwrap_or_default();
    
    let status = if pending_count > 0 { "processing" } else { "idle" };
    let last_lesson = recent_karma.get(0).and_then(|k| k["lesson"].as_str()).unwrap_or("Waiting for new insights...");

    Json(serde_json::json!({
        "status": status,
        "pending_expressions": pending_count,
        "last_insight": last_lesson,
        "message_ja": format!("自律表現パイプライン: {}。現在の洞察: {}", status, last_lesson),
        "message_en": format!("Autonomous expression pipeline {}. Current insight: {}", status, last_lesson)
    }))
}
