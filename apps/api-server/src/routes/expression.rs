use axum::{
    response::Json,
    extract::State,
};
use crate::AppState;

pub async fn expression_status(
    State(_state): State<AppState>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "idle",
        "pending_expressions": 0,
        "message_ja": "自律表現パイプライン待機中。コンテンツ生成を監視しています...",
        "message_en": "Autonomous expression pipeline idle. Monitoring for content generation..."
    }))
}
