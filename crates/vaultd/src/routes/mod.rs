pub mod health;
pub mod proxy;
pub mod stats;

use axum::{
    Router,
    routing::{get, post},
};

use crate::app::AppState;
use crate::auth::{challenge_handler, login_handler};
use crate::static_assets::login_page;

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(health::health_router())
        .merge(stats::stats_router())
        .route("/v1/proxy/{provider}/{*path}", post(proxy::proxy_handler))
        // Auth endpoints — no session required (they establish the session).
        .route("/api/auth/challenge", post(challenge_handler))
        .route("/api/auth/login", post(login_handler))
        // Dashboard HTML pages.
        .route("/login", get(login_page))
        .with_state(state)
}
