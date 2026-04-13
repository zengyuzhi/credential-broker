pub mod dashboard;
pub mod events;
pub mod health;
pub mod proxy;
pub mod stats;

use axum::{
    Router,
    http::{HeaderValue, Method, header},
    routing::{get, post},
};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    set_header::SetResponseHeaderLayer,
};

use crate::app::AppState;
use crate::auth::{challenge_handler, login_handler};
use crate::static_assets::login_page;

pub fn router(state: AppState) -> Router {
    // CORS: restrict to loopback origin only.
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::exact(
            "http://127.0.0.1:8765".parse().unwrap(),
        ))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE, header::COOKIE])
        .allow_credentials(true);

    Router::new()
        .merge(health::health_router())
        .merge(stats::stats_router())
        .route("/v1/proxy/{provider}/{*path}", post(proxy::proxy_handler))
        // Auth endpoints — no session required (they establish the session).
        .route("/api/auth/challenge", post(challenge_handler))
        .route("/api/auth/login", post(login_handler))
        // Dashboard HTML pages.
        .route("/login", get(login_page))
        .route("/", get(dashboard::home_page))
        .route("/credentials", get(dashboard::credentials_page))
        .route("/profiles", get(dashboard::profiles_page))
        .route("/sessions", get(dashboard::sessions_page))
        .route("/stats", get(dashboard::stats_page))
        // SSE live-update stream — session required, cookie sent automatically.
        .route("/api/events", get(events::events_handler))
        // API endpoints — session + CSRF protected.
        .route(
            "/api/credentials/{id}/toggle",
            post(dashboard::toggle_credential),
        )
        .with_state(state)
        // Security headers on every response.
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        // CORS layer outermost so preflight requests are handled before auth.
        .layer(cors)
}
