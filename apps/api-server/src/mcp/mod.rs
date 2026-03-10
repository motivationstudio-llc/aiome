pub mod server;
pub mod types;
pub mod client;
pub mod discovery;

use axum::{
    routing::{get, post},
    Router,
};
use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sse", get(server::sse_handler))
        .route("/messages", post(server::message_handler))
}
