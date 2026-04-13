use axum::{Json, Router, routing::get};
use serde_json::json;

pub fn health_router() -> Router {
    Router::new().route("/health", get(health))
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}
