pub mod health;
pub mod stats;

use axum::Router;

use crate::app::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(health::health_router())
        .merge(stats::stats_router())
        .with_state(state)
}
