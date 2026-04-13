use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;

use crate::app::AppState;

#[derive(Serialize)]
struct ProviderStatResponse {
    provider: String,
    requests: i64,
    prompt_tokens: i64,
    completion_tokens: i64,
    estimated_cost_usd: f64,
    last_used_at: String,
}

pub fn stats_router() -> Router<AppState> {
    Router::new().route("/stats/providers", get(stats_providers))
}

async fn stats_providers(State(state): State<AppState>) -> Json<serde_json::Value> {
    let stats = state.store.usage_stats_by_provider().await.unwrap_or_default();
    let providers: Vec<ProviderStatResponse> = stats
        .into_iter()
        .map(|s| ProviderStatResponse {
            provider: s.provider,
            requests: s.request_count,
            prompt_tokens: s.prompt_tokens,
            completion_tokens: s.completion_tokens,
            estimated_cost_usd: s.estimated_cost_usd,
            last_used_at: s.last_used_at,
        })
        .collect();
    Json(serde_json::json!({ "providers": providers }))
}
