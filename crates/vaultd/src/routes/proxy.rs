use std::time::Instant;

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use chrono::Utc;
use uuid::Uuid;
use vault_core::models::{AccessMode, UsageEvent};
use vault_policy::lease::hash_token;
use vault_providers::adapter_for;
use vault_secrets::parse_secret_ref;
use vault_telemetry::writer::TelemetryWriter;
use zeroize::Zeroizing;

use crate::app::AppState;

pub async fn proxy_handler(
    State(state): State<AppState>,
    Path((provider, path)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Bytes), (StatusCode, String)> {
    // --- 1. Authenticate via lease token ---
    let raw_token = headers
        .get("x-vault-lease-token")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                "missing x-vault-lease-token header".to_string(),
            )
        })?;

    let token_hash = hash_token(raw_token);

    let lease = state
        .store
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

    // --- 2. Resolve provider adapter ---
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

    // --- 3. Find a proxy/either binding for this provider ---
    let bindings = state
        .store
        .list_bindings_for_profile(lease.profile_id)
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to list profile bindings: {err}"),
            )
        })?;

    let binding = bindings
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

    // --- 4. Load credential ---
    let credential = state
        .store
        .get_credential(binding.credential_id)
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
                format!("credential {} not found", binding.credential_id),
            )
        })?;

    // --- 5. Retrieve secret from macOS Keychain ---
    let secret = load_secret(&credential.secret_ref).await.map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to load secret: {err}"),
        )
    })?;

    // --- 6. Build and send upstream request ---
    let upstream_url = format!("{base_url}/{path}");

    let mut req_builder = state.http_client.post(&upstream_url).body(body.to_vec());

    // Forward content-type if provided.
    if let Some(ct) = headers.get("content-type") {
        req_builder = req_builder.header("content-type", ct);
    }

    // Inject provider-specific auth headers.
    req_builder = match provider.as_str() {
        "anthropic" => req_builder
            .header("x-api-key", secret.as_str())
            .header("anthropic-version", "2023-06-01"),
        _ => {
            // Hold the `Bearer <api-key>` header value in a Zeroizing<String>
            // so the allocation is wiped on drop once reqwest copies into its
            // internal header map. Audit ZA-0003.
            let auth_header: Zeroizing<String> =
                Zeroizing::new(format!("Bearer {}", secret.as_str()));
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

    // --- 7. Parse usage ---
    let parsed = adapter.parse_usage_from_response(&path, status_code.as_u16(), &response_body);

    // --- 8. Record telemetry ---
    let telemetry = TelemetryWriter::new(state.store.clone());
    let event = UsageEvent {
        id: Uuid::new_v4(),
        provider: provider.clone(),
        credential_id: credential.id,
        lease_id: Some(lease.id),
        agent_name: lease.agent_name.clone(),
        project: lease.project.clone(),
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
    };

    if let Err(err) = telemetry.write_usage_event(&event).await {
        tracing::warn!("failed to record proxy usage event: {err}");
    }

    // --- 9. Return upstream response ---
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
        let _ = secret_ref;
        anyhow::bail!("proxy secret loading is only implemented for macOS");
    }
}
