use std::time::Instant;

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use chrono::Utc;
use uuid::Uuid;
use vault_core::{
    models::{AccessMode, Credential, Lease, ProfileBinding, Session, UsageEvent},
    provider::ProviderAdapter,
};
use vault_db::{Store, sessions::SessionGrantCandidate};
use vault_policy::{lease::hash_token, service::PolicyService, session::check_session_active};
use vault_providers::adapter_for;
use vault_secrets::parse_secret_ref;
use vault_telemetry::writer::TelemetryWriter;
use zeroize::Zeroizing;

use crate::app::AppState;

/// Attribution context shared by both auth branches. Carries enough data to
/// build a complete `UsageEvent`, credit the right session/lease, and
/// (session branch only) increment the per-grant quota counter.
struct AttributionCtx {
    agent_name: String,
    project: Option<String>,
    lease_id: Option<Uuid>,
    session_id: Option<Uuid>,
    bundle_id: Option<Uuid>,
    /// Phase 1.1: the specific grant selected to authorize this request.
    /// Only set on the session branch; used to bump `session_grants.request_count`.
    grant_id: Option<Uuid>,
}

/// Result of auth resolution: a credential to use plus attribution info.
struct ResolvedAuth {
    credential_id: Uuid,
    attribution: AttributionCtx,
}

pub async fn proxy_handler(
    State(state): State<AppState>,
    Path((provider, path)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Bytes), (StatusCode, String)> {
    // --- 1. Resolve the provider adapter up-front (fail fast) ---
    let adapter = adapter_for(&provider).map_err(|err| {
        (
            StatusCode::BAD_GATEWAY,
            format!("unsupported provider: {err}"),
        )
    })?;
    if !adapter.supports_proxy() {
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("provider {provider} does not support proxy mode"),
        ));
    }
    let base_url = adapter.upstream_base_url().ok_or_else(|| {
        (
            StatusCode::BAD_GATEWAY,
            format!("provider {provider} has no upstream base URL configured"),
        )
    })?;

    // --- 2. Resolve auth: prefer Authorization bearer (session), fall back
    //        to x-vault-lease-token (legacy lease path) ---
    let resolved = resolve_auth(&state.store, &headers, &provider).await?;

    // --- 3. Load the credential identified by whichever auth path ran ---
    let credential = state
        .store
        .get_credential(resolved.credential_id)
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to load credential: {err}"),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("credential {} not found", resolved.credential_id),
            )
        })?;

    // --- 4. Dispatch the upstream request and record telemetry ---
    dispatch_and_record(
        state,
        adapter.as_ref(),
        provider,
        base_url,
        path,
        headers,
        body,
        credential,
        resolved.attribution,
    )
    .await
}

/// Inspect the headers and dispatch to either the session branch (preferred,
/// `Authorization: Bearer <session-token>`) or the lease branch (compat,
/// `x-vault-lease-token`). Returns the credential to use plus attribution.
async fn resolve_auth(
    store: &Store,
    headers: &HeaderMap,
    provider: &str,
) -> Result<ResolvedAuth, (StatusCode, String)> {
    if let Some(bearer) = extract_bearer(headers) {
        return resolve_session(store, provider, bearer, headers).await;
    }
    if let Some(lease_token) = headers
        .get("x-vault-lease-token")
        .and_then(|v| v.to_str().ok())
    {
        return resolve_lease(store, provider, lease_token).await;
    }
    Err((
        StatusCode::UNAUTHORIZED,
        "expected Authorization: Bearer <session-token> or x-vault-lease-token".to_string(),
    ))
}

/// Phase 1.1: optional narrowing headers a caller can send to pick a specific
/// connector or capability when a session has multiple eligible grants for
/// the same provider.
fn extract_hint<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn extract_bearer(headers: &HeaderMap) -> Option<&str> {
    let raw = headers.get("authorization")?.to_str().ok()?;
    // Case-insensitive "Bearer " prefix, RFC 7235 §2.1.
    let prefix_len = 7;
    if raw.len() > prefix_len && raw[..prefix_len].eq_ignore_ascii_case("Bearer ") {
        Some(raw[prefix_len..].trim())
    } else {
        None
    }
}

async fn resolve_session(
    store: &Store,
    provider: &str,
    raw_token: &str,
    headers: &HeaderMap,
) -> Result<ResolvedAuth, (StatusCode, String)> {
    let token_hash = hash_token(raw_token);
    let session: Session = store
        .get_broker_session_by_token_hash(&token_hash)
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to look up session: {err}"),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                "invalid session token".to_string(),
            )
        })?;

    check_session_active(&session).map_err(|err| (StatusCode::UNAUTHORIZED, err.to_string()))?;

    let candidates: Vec<SessionGrantCandidate> = store
        .list_session_proxy_candidates(session.id, provider)
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to resolve session grant: {err}"),
            )
        })?;

    let selected = pick_candidate(candidates, provider, headers)?;

    // Defensive recheck: list_session_proxy_candidates filters on enabled
    // at every cascade level but not on grant.expires_at. PolicyService
    // catches expired grants that passed the SQL filter.
    PolicyService::default()
        .check_grant_active(&selected.grant)
        .map_err(|err| (StatusCode::FORBIDDEN, err.to_string()))?;

    // Phase 1.1: per-grant quota. `session_grant_request_count` is the count
    // *before* this request; reject if incrementing would exceed the cap.
    if let Some(max) = selected.grant.max_requests
        && selected.session_grant_request_count >= max
    {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            format!(
                "grant {} quota exhausted ({max} requests for this session)",
                selected.grant.id
            ),
        ));
    }

    Ok(ResolvedAuth {
        credential_id: selected.connector.credential_id,
        attribution: AttributionCtx {
            agent_name: session.agent_name.clone(),
            project: session.project.clone(),
            lease_id: None,
            session_id: Some(session.id),
            bundle_id: session.bundle_id,
            grant_id: Some(selected.grant.id),
        },
    })
}

/// Phase 1.1: pick the single grant that authorizes this request. When the
/// session has multiple eligible grants for the same provider, the caller
/// must narrow with `X-Vault-Connector: <name>` and/or `X-Vault-Capability: <name>`.
fn pick_candidate(
    candidates: Vec<SessionGrantCandidate>,
    provider: &str,
    headers: &HeaderMap,
) -> Result<SessionGrantCandidate, (StatusCode, String)> {
    if candidates.is_empty() {
        return Err((
            StatusCode::FORBIDDEN,
            format!("no grant authorizes provider {provider} for this session"),
        ));
    }

    let connector_hint = extract_hint(headers, "x-vault-connector");
    let capability_hint = extract_hint(headers, "x-vault-capability");

    let filtered: Vec<SessionGrantCandidate> = candidates
        .into_iter()
        .filter(|c| connector_hint.is_none_or(|name| c.connector.name == name))
        .filter(|c| capability_hint.is_none_or(|name| c.capability.name == name))
        .collect();

    match filtered.len() {
        0 => Err((
            StatusCode::FORBIDDEN,
            match (connector_hint, capability_hint) {
                (Some(_), Some(_)) => {
                    "no grant matches the specified X-Vault-Connector and X-Vault-Capability"
                        .to_string()
                }
                (Some(_), None) => "no grant matches the specified X-Vault-Connector".to_string(),
                (None, Some(_)) => "no grant matches the specified X-Vault-Capability".to_string(),
                // Shouldn't happen — empty with no hints implies candidates
                // was already empty, which we handled above.
                (None, None) => format!("no grant authorizes provider {provider} for this session"),
            },
        )),
        1 => Ok(filtered.into_iter().next().expect("len == 1")),
        _ => {
            let hint: Vec<String> = filtered
                .iter()
                .map(|c| format!("{}/{}", c.connector.name, c.capability.name))
                .collect();
            Err((
                StatusCode::CONFLICT,
                format!(
                    "session has multiple grants for provider {provider}; \
                     disambiguate with X-Vault-Connector and/or X-Vault-Capability. \
                     Candidates: {}",
                    hint.join(", ")
                ),
            ))
        }
    }
}

async fn resolve_lease(
    store: &Store,
    provider: &str,
    raw_token: &str,
) -> Result<ResolvedAuth, (StatusCode, String)> {
    let token_hash = hash_token(raw_token);
    let lease: Lease = store
        .get_lease_by_token_hash(&token_hash)
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to look up lease: {err}"),
            )
        })?
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "invalid lease token".to_string()))?;

    if lease.expires_at < Utc::now() {
        return Err((
            StatusCode::UNAUTHORIZED,
            "lease token has expired".to_string(),
        ));
    }

    let bindings = store
        .list_bindings_for_profile(lease.profile_id)
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to list profile bindings: {err}"),
            )
        })?;

    let binding: ProfileBinding = bindings
        .into_iter()
        .find(|b| {
            b.provider == provider && matches!(b.mode, AccessMode::Proxy | AccessMode::Either)
        })
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                format!("no proxy binding found for provider {provider} in this profile"),
            )
        })?;

    Ok(ResolvedAuth {
        credential_id: binding.credential_id,
        attribution: AttributionCtx {
            agent_name: lease.agent_name.clone(),
            project: lease.project.clone(),
            lease_id: Some(lease.id),
            session_id: None,
            bundle_id: None,
            grant_id: None,
        },
    })
}

#[allow(clippy::too_many_arguments)]
async fn dispatch_and_record(
    state: AppState,
    adapter: &dyn ProviderAdapter,
    provider: String,
    base_url: &str,
    path: String,
    headers: HeaderMap,
    body: Bytes,
    credential: Credential,
    attribution: AttributionCtx,
) -> Result<(StatusCode, Bytes), (StatusCode, String)> {
    // --- Retrieve secret from macOS Keychain ---
    let secret = load_secret(&credential.secret_ref).await.map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to load secret: {err}"),
        )
    })?;

    // --- Build and send upstream request ---
    let upstream_url = format!("{base_url}/{path}");
    let mut req_builder = state.http_client.post(&upstream_url).body(body.to_vec());
    if let Some(ct) = headers.get("content-type") {
        req_builder = req_builder.header("content-type", ct);
    }
    req_builder = match provider.as_str() {
        "anthropic" => req_builder
            .header("x-api-key", secret.as_str())
            .header("anthropic-version", "2023-06-01"),
        _ => {
            // Hold the `Bearer <api-key>` header value in a Zeroizing<String>
            // so the allocation is wiped on drop once reqwest copies into its
            // internal header map. Pre-allocate to the exact final size so
            // `push_str` never reallocates and leaves un-zeroized fragments
            // in the allocator's free list. Audit ZA-0003.
            const BEARER_PREFIX: &str = "Bearer ";
            let mut auth_header =
                Zeroizing::new(String::with_capacity(BEARER_PREFIX.len() + secret.len()));
            auth_header.push_str(BEARER_PREFIX);
            auth_header.push_str(secret.as_str());
            req_builder.header("authorization", auth_header.as_str())
        }
    };

    let start = Instant::now();
    let upstream_response = req_builder.send().await.map_err(|err| {
        (
            StatusCode::BAD_GATEWAY,
            format!("upstream request failed: {err}"),
        )
    })?;
    let status_code = upstream_response.status();
    let response_body = upstream_response.bytes().await.map_err(|err| {
        (
            StatusCode::BAD_GATEWAY,
            format!("failed to read upstream response body: {err}"),
        )
    })?;
    // Saturate instead of truncate: an outlier latency above i64::MAX ms
    // deserves a sticky max value, not a silent wrap. Audit SE-08.
    let latency_ms = i64::try_from(start.elapsed().as_millis()).unwrap_or(i64::MAX);

    // --- Parse usage and record telemetry ---
    let parsed = adapter.parse_usage_from_response(&path, status_code.as_u16(), &response_body);
    let telemetry = TelemetryWriter::new(state.store.clone());
    let event = UsageEvent {
        id: Uuid::new_v4(),
        provider: provider.clone(),
        credential_id: credential.id,
        lease_id: attribution.lease_id,
        agent_name: attribution.agent_name,
        project: attribution.project,
        mode: AccessMode::Proxy,
        operation: parsed.operation.clone(),
        endpoint: parsed.endpoint.clone().or_else(|| Some(path.clone())),
        model: parsed.model.clone(),
        request_count: 1,
        prompt_tokens: parsed.prompt_tokens,
        completion_tokens: parsed.completion_tokens,
        total_tokens: parsed.total_tokens,
        // Convert adapter-supplied f64 USD → integer microdollars at the DB
        // boundary. Audit SE-09.
        estimated_cost_micros: parsed
            .estimated_cost_usd
            .map(|usd| (usd * 1_000_000.0) as i64),
        status_code: Some(status_code.as_u16() as i64),
        success: status_code.is_success(),
        latency_ms,
        error_text: if status_code.is_success() {
            None
        } else {
            Some(format!("upstream returned HTTP {}", status_code.as_u16()))
        },
        created_at: Utc::now(),
        session_id: attribution.session_id,
        bundle_id: attribution.bundle_id,
    };
    if let Err(err) = telemetry.write_usage_event(&event).await {
        tracing::warn!("failed to record proxy usage event: {err}");
    }

    let _ = state.store.touch_credential_last_used(credential.id).await;

    // Session counters (fire-and-forget). Session-wide counter is an
    // observability metric; the per-grant counter enforces `max_requests`
    // on the *next* request — we increment only on success so a failed
    // upstream call doesn't permanently erode the caller's quota.
    if let (Some(session_id), Some(grant_id)) = (attribution.session_id, attribution.grant_id) {
        let _ = state
            .store
            .increment_session_request_count(session_id)
            .await;
        if status_code.is_success() {
            let _ = state
                .store
                .increment_session_grant_request_count(session_id, grant_id)
                .await;
        }
    }

    Ok((status_code, response_body))
}

/// Load a secret from the platform secret store, parsing the `secret_ref` format `"service:account"`.
async fn load_secret(secret_ref: &str) -> anyhow::Result<Zeroizing<String>> {
    #[cfg(target_os = "macos")]
    {
        use vault_secrets::{MacOsKeychainStore, SecretStore};

        let (service, account) = parse_secret_ref(secret_ref)?;

        let store = MacOsKeychainStore;
        let secret = store.get(service, account).await?;
        Ok(secret)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = parse_secret_ref(secret_ref)?;
        anyhow::bail!("secret retrieval is only implemented for macOS")
    }
}
