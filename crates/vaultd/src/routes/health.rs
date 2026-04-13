use axum::{Json, Router, routing::get};
use serde_json::json;

use crate::app::AppState;

pub fn health_router() -> Router<AppState> {
    Router::new().route("/health", get(health))
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}
