pub mod health;
pub mod proxy;
pub mod stats;

use axum::Router;
use axum::routing::post;

use crate::app::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(health::health_router())
        .merge(stats::stats_router())
        .route("/v1/proxy/{provider}/{*path}", post(proxy::proxy_handler))
        .with_state(state)
}
