//! Dashboard home, credentials, profiles, and sessions page handlers.

use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::{validate_csrf, AuthSession};

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

    let recent_events = match store.list_usage_events(5).await {
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

// ---------------------------------------------------------------------------
// Credentials page
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "credentials.html")]
pub struct CredentialsTemplate {
    pub credentials: Vec<vault_core::models::Credential>,
    pub csrf_token: String,
}

#[derive(Template)]
#[template(path = "credential_row.html")]
pub struct CredentialRowTemplate {
    pub credential: vault_core::models::Credential,
}

/// `GET /credentials` — credentials list page (requires active session).
pub async fn credentials_page(auth: AuthSession, State(state): State<AppState>) -> Response {
    let store = &state.store;

    let credentials = match store.list_credentials().await {
        Ok(list) => list,
        Err(err) => {
            tracing::error!("failed to list credentials: {err}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load credentials".to_string(),
            )
                .into_response();
        }
    };

    let csrf_token = auth.session.csrf_token.clone().unwrap_or_default();
    let tmpl = CredentialsTemplate {
        credentials,
        csrf_token,
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

// ---------------------------------------------------------------------------
// Profiles page
// ---------------------------------------------------------------------------

pub struct ProfileWithBindings {
    pub profile: vault_core::models::Profile,
    pub bindings: Vec<vault_core::models::ProfileBinding>,
}

#[derive(Template)]
#[template(path = "profiles.html")]
pub struct ProfilesTemplate {
    pub profiles: Vec<ProfileWithBindings>,
}

/// `GET /profiles` — profiles list page (requires active session).
pub async fn profiles_page(_auth: AuthSession, State(state): State<AppState>) -> Response {
    let store = &state.store;

    let profile_list = match store.list_profiles().await {
        Ok(list) => list,
        Err(err) => {
            tracing::error!("failed to list profiles: {err}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load profiles".to_string(),
            )
                .into_response();
        }
    };

    let mut profiles: Vec<ProfileWithBindings> = Vec::with_capacity(profile_list.len());
    for profile in profile_list {
        let bindings = match store.list_bindings_for_profile(profile.id).await {
            Ok(b) => b,
            Err(err) => {
                tracing::error!("failed to list bindings for profile {}: {err}", profile.id);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to load profile bindings".to_string(),
                )
                    .into_response();
            }
        };
        profiles.push(ProfileWithBindings { profile, bindings });
    }

    let tmpl = ProfilesTemplate { profiles };

    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("template error: {err}"),
        )
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// Sessions page
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "sessions.html")]
pub struct SessionsTemplate {
    pub active_leases: Vec<vault_core::models::Lease>,
    pub expired_leases: Vec<vault_core::models::Lease>,
}

/// `GET /sessions` — sessions list page (requires active session).
pub async fn sessions_page(_auth: AuthSession, State(state): State<AppState>) -> Response {
    let store = &state.store;

    let active_leases = match store.list_active_leases().await {
        Ok(list) => list,
        Err(err) => {
            tracing::error!("failed to list active leases: {err}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load active leases".to_string(),
            )
                .into_response();
        }
    };

    let expired_leases = match store.list_expired_leases(50).await {
        Ok(list) => list,
        Err(err) => {
            tracing::error!("failed to list expired leases: {err}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load expired leases".to_string(),
            )
                .into_response();
        }
    };

    let tmpl = SessionsTemplate {
        active_leases,
        expired_leases,
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

// ---------------------------------------------------------------------------
// Stats page
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub provider: Option<String>,
}

#[derive(Template)]
#[template(path = "stats.html")]
pub struct StatsTemplate {
    pub provider_stats: Vec<vault_db::ProviderStats>,
    pub recent_events: Vec<vault_core::models::UsageEvent>,
    pub providers: Vec<String>,
    pub selected_provider: String,
}

/// `GET /stats` — usage statistics page with optional provider filter.
pub async fn stats_page(
    _auth: AuthSession,
    State(state): State<AppState>,
    Query(params): Query<StatsQuery>,
) -> Response {
    let store = &state.store;

    let all_stats = match store.usage_stats_by_provider().await {
        Ok(s) => s,
        Err(err) => {
            tracing::error!("failed to load provider stats: {err}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load usage stats".to_string(),
            )
                .into_response();
        }
    };

    let providers: Vec<String> = all_stats.iter().map(|s| s.provider.clone()).collect();

    let selected_provider = params
        .provider
        .clone()
        .unwrap_or_default()
        .trim()
        .to_string();

    let (provider_stats, recent_events) = if selected_provider.is_empty() {
        let events = match store.list_usage_events(20).await {
            Ok(e) => e,
            Err(err) => {
                tracing::error!("failed to list usage events: {err}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to load recent events".to_string(),
                )
                    .into_response();
            }
        };
        (all_stats, events)
    } else {
        let stats = match store.usage_stats_for_provider(&selected_provider).await {
            Ok(opt) => opt.map(|s| vec![s]).unwrap_or_default(),
            Err(err) => {
                tracing::error!("failed to load stats for provider {selected_provider}: {err}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to load provider stats".to_string(),
                )
                    .into_response();
            }
        };
        let events = match store
            .list_usage_events_for_provider(&selected_provider, 20)
            .await
        {
            Ok(e) => e,
            Err(err) => {
                tracing::error!("failed to list events for provider {selected_provider}: {err}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to load recent events".to_string(),
                )
                    .into_response();
            }
        };
        (stats, events)
    };

    let tmpl = StatsTemplate {
        provider_stats,
        recent_events,
        providers,
        selected_provider,
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

// ---------------------------------------------------------------------------
// Credentials toggle
// ---------------------------------------------------------------------------

/// `POST /api/credentials/:id/toggle` — enable/disable a credential (htmx partial response).
pub async fn toggle_credential(
    auth: AuthSession,
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    validate_csrf(&headers, &auth.session)?;

    let uuid = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, format!("invalid credential id: {id}")))?;

    let store = &state.store;

    let credential = store
        .get_credential(uuid)
        .await
        .map_err(|err| {
            tracing::error!("failed to fetch credential {uuid}: {err}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to load credential".to_string(),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("credential {id} not found")))?;

    store
        .set_credential_enabled(uuid, !credential.enabled)
        .await
        .map_err(|err| {
            tracing::error!("failed to toggle credential {uuid}: {err}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to update credential".to_string(),
            )
        })?;

    // Reload updated credential to reflect new state in the partial.
    let updated = store
        .get_credential(uuid)
        .await
        .map_err(|err| {
            tracing::error!("failed to reload credential {uuid}: {err}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to reload credential".to_string(),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("credential {id} not found after toggle")))?;

    let tmpl = CredentialRowTemplate { credential: updated };

    tmpl.render()
        .map(|html| Html(html).into_response())
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("template error: {err}"),
            )
        })
}
