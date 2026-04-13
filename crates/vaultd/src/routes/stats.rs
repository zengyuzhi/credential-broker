use axum::{Json, Router, routing::get};
use serde_json::json;

use crate::app::AppState;

pub fn stats_router() -> Router<AppState> {
    Router::new().route("/stats/providers", get(provider_stats))
}

async fn provider_stats() -> Json<serde_json::Value> {
    Json(json!({ "providers": [] }))
}
