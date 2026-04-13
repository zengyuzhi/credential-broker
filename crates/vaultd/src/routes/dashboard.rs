//! Dashboard home page handler.

use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

use crate::app::AppState;
use crate::auth::AuthSession;

#[derive(Template)]
#[template(path = "home.html")]
pub struct HomeTemplate {
    pub credential_count: usize,
    pub profile_count: usize,
    pub active_lease_count: i64,
    pub provider_stats: Vec<vault_db::ProviderStats>,
    pub recent_events: Vec<vault_core::models::UsageEvent>,
}

/// `GET /` — dashboard home page (requires active session).
pub async fn home_page(_auth: AuthSession, State(state): State<AppState>) -> Response {
    let store = &state.store;

    let credential_count = match store.list_credentials().await {
        Ok(list) => list.len(),
        Err(err) => {
            tracing::error!("failed to list credentials: {err}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load credential data".to_string(),
            )
                .into_response();
        }
    };

    let profile_count = match store.list_profiles().await {
        Ok(list) => list.len(),
        Err(err) => {
            tracing::error!("failed to list profiles: {err}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load profile data".to_string(),
            )
                .into_response();
        }
    };

    let active_lease_count = match store.count_active_leases().await {
        Ok(n) => n,
        Err(err) => {
            tracing::error!("failed to count active leases: {err}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load lease data".to_string(),
            )
                .into_response();
        }
    };

    let provider_stats = match store.usage_stats_by_provider().await {
        Ok(stats) => stats,
        Err(err) => {
            tracing::error!("failed to load provider stats: {err}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load usage stats".to_string(),
            )
                .into_response();
        }
    };

    let recent_events = match store.list_usage_events(10).await {
        Ok(events) => events,
        Err(err) => {
            tracing::error!("failed to list usage events: {err}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load recent events".to_string(),
            )
                .into_response();
        }
    };

    let tmpl = HomeTemplate {
        credential_count,
        profile_count,
        active_lease_count,
        provider_stats,
        recent_events,
    };

    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("template error: {err}"),
        )
            .into_response(),
    }
}
