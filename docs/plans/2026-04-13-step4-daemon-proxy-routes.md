# Daemon Proxy Routes Implementation Plan

> **For agentic workers:** Use subagent-driven-development to implement this plan task-by-task.

**Goal:** Enable vaultd to forward HTTP requests to upstream providers on behalf of agents, authenticating via lease tokens and recording usage events.

**Architecture:** Agents set `VAULT_LEASE_TOKEN` in their HTTP headers. vaultd validates the lease, resolves the profile's proxy bindings, injects the real API key, forwards the request to the upstream provider via reqwest, parses the response for usage data, records a telemetry event, and returns the response to the agent. The agent never sees the raw API key.

**Tech Stack:** Rust 2024, axum, reqwest, sqlx + SQLite, tokio.

**Prerequisites:** The telemetry plan (step 5) must be completed first — this plan depends on `insert_usage_event` and `TelemetryWriter`.

**Out of scope:**
- Streaming/SSE proxy (forward buffered responses only)
- WebSocket proxy
- Request/response body transformation
- Rate limiting (future work)

---

## Shared conventions for every task

- Run `cargo fmt --all` after each task that changes Rust files
- Run the narrowest possible test first, then `cargo test`
- The daemon binds to `127.0.0.1:8765`

---

### Task 1: Add lease lookup by token hash to vault-db

**Objective:** vaultd needs to authenticate incoming requests by looking up a lease from its hashed session token.

**Files:**
- Modify: `crates/vault-db/src/leases.rs`
- Test: `crates/vault-db/tests/repositories.rs`

- [ ] **Step 1: Write a failing test**

File: `crates/vault-db/tests/repositories.rs`

Add:

```rust
#[tokio::test]
async fn get_lease_by_token_hash_should_return_matching_lease() {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite:{}", dir.path().join("test.db").display());
    let store = Store::connect(&url).await.unwrap();

    // Create a profile first (FK constraint).
    let profile = vault_core::models::Profile {
        id: uuid::Uuid::new_v4(),
        name: "coding".to_string(),
        description: None,
        default_project: None,
        created_at: chrono::Utc::now(),
    };
    store.insert_profile(&profile).await.unwrap();

    let (lease, raw_token) = vault_policy::lease::issue_lease(profile.id, "demo", None, 60);
    store.insert_lease(&lease).await.unwrap();

    let found = store
        .get_lease_by_token_hash(&lease.session_token_hash)
        .await
        .unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, lease.id);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vault-db get_lease_by_token_hash -- --nocapture`

Expected: fail because `get_lease_by_token_hash` does not exist.

- [ ] **Step 3: Implement the lookup**

Add to `crates/vault-db/src/leases.rs`:

```rust
pub async fn get_lease_by_token_hash(
    &self,
    token_hash: &str,
) -> anyhow::Result<Option<Lease>> {
    let row = sqlx::query_as::<_, LeaseRow>(
        "SELECT id, profile_id, agent_name, project, issued_at, expires_at, session_token_hash \
         FROM leases WHERE session_token_hash = ?",
    )
    .bind(token_hash)
    .fetch_optional(&self.pool)
    .await
    .context("failed to look up lease by token hash")?;

    row.map(|r| r.try_into()).transpose()
}
```

If `LeaseRow` does not exist yet, add it following the same pattern as `CredentialRow` in `credentials.rs`:

```rust
#[derive(sqlx::FromRow)]
struct LeaseRow {
    id: String,
    profile_id: String,
    agent_name: String,
    project: Option<String>,
    issued_at: String,
    expires_at: String,
    session_token_hash: String,
}

impl TryFrom<LeaseRow> for Lease {
    type Error = anyhow::Error;
    fn try_from(row: LeaseRow) -> anyhow::Result<Self> {
        Ok(Lease {
            id: uuid::Uuid::parse_str(&row.id)?,
            profile_id: uuid::Uuid::parse_str(&row.profile_id)?,
            agent_name: row.agent_name,
            project: row.project,
            issued_at: chrono::DateTime::parse_from_rfc3339(&row.issued_at)?.with_timezone(&chrono::Utc),
            expires_at: chrono::DateTime::parse_from_rfc3339(&row.expires_at)?.with_timezone(&chrono::Utc),
            session_token_hash: row.session_token_hash,
        })
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vault-db get_lease_by_token_hash -- --nocapture`

Expected: pass.

- [ ] **Step 5: Commit**

Run: `git add crates/vault-db && git commit -m "feat: add lease lookup by token hash"`

---

### Task 2: Enable proxy bindings in profile CLI

**Objective:** Remove the Phase 1 guard that prevents creating proxy bindings.

**Files:**
- Modify: `crates/vault-cli/src/commands/profile.rs`
- Test: `crates/vault-cli/src/commands/profile.rs`

- [ ] **Step 1: Find and remove the bail**

In `crates/vault-cli/src/commands/profile.rs`, find:

```rust
if matches!(mode, AccessMode::Proxy) {
    bail!("proxy bindings are not enabled in Phase 1");
}
```

Remove this block entirely. Proxy is now supported.

- [ ] **Step 2: Run tests**

Run: `cargo test -p vault-cli -- --nocapture`

Expected: pass (existing tests don't create proxy bindings).

- [ ] **Step 3: Commit**

Run: `git add crates/vault-cli && git commit -m "feat: enable proxy bindings in profile CLI"`

---

### Task 3: Add AppState with DB and reqwest to vaultd

**Objective:** Give vaultd a real application state with a database pool and HTTP client for proxying.

**Files:**
- Modify: `crates/vaultd/Cargo.toml`
- Modify: `crates/vaultd/src/app.rs`
- Modify: `crates/vaultd/src/main.rs`

- [ ] **Step 1: Add dependencies to vaultd**

Add to `crates/vaultd/Cargo.toml` under `[dependencies]`:

```toml
reqwest.workspace = true
vault-core = { path = "../vault-core" }
vault-db = { path = "../vault-db" }
vault-policy = { path = "../vault-policy" }
vault-providers = { path = "../vault-providers" }
vault-secrets = { path = "../vault-secrets" }
vault-telemetry = { path = "../vault-telemetry" }
```

- [ ] **Step 2: Update AppState**

File: `crates/vaultd/src/app.rs`

```rust
use std::sync::Arc;

use vault_db::Store;

#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub http_client: reqwest::Client,
}

impl AppState {
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        let store = Store::connect(database_url).await?;
        let http_client = reqwest::Client::new();
        Ok(Self { store, http_client })
    }
}
```

- [ ] **Step 3: Update main.rs to use real state**

File: `crates/vaultd/src/main.rs`

Update to construct `AppState` with a real DB connection:

```rust
use crate::app::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("VAULT_DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:.local/vault.db".to_string());
    let state = AppState::new(&database_url).await?;

    let app = routes::router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8765").await?;
    tracing::info!("vaultd listening on 127.0.0.1:8765");
    axum::serve(listener, app).await?;
    Ok(())
}
```

Update `routes/mod.rs` to accept `AppState` and thread it through sub-routers.

- [ ] **Step 4: Verify it compiles**

Run: `cargo build -p vaultd`

Expected: compiles (runtime behavior unchanged — health and stats routes don't use state yet).

- [ ] **Step 5: Commit**

Run: `git add crates/vaultd && git commit -m "feat: add AppState with DB and reqwest to vaultd"`

---

### Task 4: Implement the proxy route

**Objective:** Add `POST /v1/proxy/:provider/*path` that authenticates via lease token, injects credentials, and forwards to upstream.

**Files:**
- Create: `crates/vaultd/src/routes/proxy.rs`
- Modify: `crates/vaultd/src/routes/mod.rs`
- Test: `crates/vaultd/src/routes/proxy.rs`

- [ ] **Step 1: Write a failing unit test for lease authentication extraction**

File: `crates/vaultd/src/routes/proxy.rs`

Add a test for extracting and hashing the lease token from the request header:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_token_hash_from_header() {
        let raw = "abc123-def456";
        let hash = vault_policy::lease::hash_token(raw);
        assert!(!hash.is_empty());
        assert_ne!(hash, raw);
    }
}
```

- [ ] **Step 2: Implement the proxy route**

File: `crates/vaultd/src/routes/proxy.rs`

```rust
use std::sync::Arc;

use anyhow::Context;
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use uuid::Uuid;
use vault_core::models::{AccessMode, UsageEvent};
use vault_policy::lease::hash_token;
use vault_providers::adapter_for;
use vault_secrets::{KEYCHAIN_SERVICE_NAME, SecretStore};
use vault_telemetry::TelemetryWriter;

use crate::app::AppState;

const LEASE_TOKEN_HEADER: &str = "x-vault-lease-token";

pub async fn proxy_handler(
    State(state): State<AppState>,
    Path((provider, path)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    match handle_proxy(&state, &provider, &path, &headers, &body).await {
        Ok(resp) => resp,
        Err(err) => {
            tracing::error!("proxy error: {err:#}");
            (StatusCode::BAD_GATEWAY, format!("proxy error: {err}")).into_response()
        }
    }
}

async fn handle_proxy(
    state: &AppState,
    provider: &str,
    path: &str,
    headers: &HeaderMap,
    body: &Bytes,
) -> anyhow::Result<Response> {
    // 1. Authenticate via lease token.
    let raw_token = headers
        .get(LEASE_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
        .context("missing x-vault-lease-token header")?;
    let token_hash = hash_token(raw_token);
    let lease = state
        .store
        .get_lease_by_token_hash(&token_hash)
        .await?
        .context("invalid or expired lease token")?;

    // Check expiration.
    if lease.expires_at < Utc::now() {
        anyhow::bail!("lease expired at {}", lease.expires_at);
    }

    // 2. Resolve the provider adapter and upstream URL.
    let adapter = adapter_for(provider)?;
    let base_url = adapter
        .upstream_base_url()
        .context("provider does not support proxy mode")?;

    // 3. Look up the proxy binding for this profile + provider.
    let bindings = state.store.list_bindings_for_profile(lease.profile_id).await?;
    let binding = bindings
        .iter()
        .find(|b| b.provider == provider && matches!(b.mode, AccessMode::Proxy | AccessMode::Either))
        .context("no proxy binding found for this provider in the lease's profile")?;

    // 4. Resolve credential and inject auth header.
    let credential = state
        .store
        .get_credential(binding.credential_id)
        .await?
        .context("credential not found for binding")?;

    let (_, account) = credential
        .secret_ref
        .split_once(':')
        .context("invalid secret_ref format")?;

    #[cfg(target_os = "macos")]
    let secret_value = {
        let keychain = vault_secrets::MacOsKeychainStore;
        keychain.get(KEYCHAIN_SERVICE_NAME, account).await?
    };

    #[cfg(not(target_os = "macos"))]
    let secret_value: String = {
        anyhow::bail!("proxy mode requires macOS Keychain in current implementation");
    };

    // 5. Build and send upstream request.
    let upstream_url = format!("{base_url}/{path}");
    let start = std::time::Instant::now();

    let mut req = state
        .http_client
        .post(&upstream_url)
        .header("content-type", "application/json");

    // Inject provider-specific auth header.
    match provider {
        "openai" | "openrouter" => {
            req = req.header("authorization", format!("Bearer {secret_value}"));
        }
        "anthropic" => {
            req = req
                .header("x-api-key", &secret_value)
                .header("anthropic-version", "2023-06-01");
        }
        _ => {
            req = req.header("authorization", format!("Bearer {secret_value}"));
        }
    }

    let upstream_response = req.body(body.to_vec()).send().await?;
    let elapsed = start.elapsed();

    let status = upstream_response.status();
    let response_bytes = upstream_response.bytes().await?;

    // 6. Parse usage from response.
    let parsed = adapter.parse_usage_from_response(path, status.as_u16(), &response_bytes);

    // 7. Record telemetry.
    let event = UsageEvent {
        id: Uuid::new_v4(),
        provider: provider.to_string(),
        credential_id: credential.id,
        lease_id: Some(lease.id),
        agent_name: lease.agent_name.clone(),
        project: lease.project.clone(),
        mode: AccessMode::Proxy,
        operation: parsed.operation,
        endpoint: parsed.endpoint,
        model: parsed.model,
        request_count: 1,
        prompt_tokens: parsed.prompt_tokens,
        completion_tokens: parsed.completion_tokens,
        total_tokens: parsed.total_tokens,
        estimated_cost_usd: parsed.estimated_cost_usd,
        status_code: Some(status.as_u16() as i64),
        success: status.is_success(),
        latency_ms: elapsed.as_millis() as i64,
        error_text: if status.is_success() { None } else { Some(String::from_utf8_lossy(&response_bytes).into_owned()) },
        created_at: Utc::now(),
    };

    let writer = TelemetryWriter::new(state.store.clone());
    if let Err(err) = writer.write_usage_event(&event).await {
        tracing::warn!("failed to record proxy usage event: {err}");
    }

    // 8. Return upstream response to agent.
    let mut response = Response::builder().status(status);
    response = response.header("content-type", "application/json");
    Ok(response.body(axum::body::Body::from(response_bytes)).unwrap())
}
```

- [ ] **Step 3: Wire into the router**

In `crates/vaultd/src/routes/mod.rs`, add:

```rust
pub mod proxy;
```

And merge the proxy route into the router:

```rust
.route("/v1/proxy/{provider}/{*path}", axum::routing::post(proxy::proxy_handler))
```

Pass `AppState` via `.with_state(state)`.

- [ ] **Step 4: Verify compilation**

Run: `cargo build -p vaultd`

Expected: compiles.

- [ ] **Step 5: Commit**

Run: `git add crates/vaultd && git commit -m "feat: add proxy route with lease auth and telemetry"`

---

### Task 5: Wire real stats into vaultd stats route

**Objective:** Make `GET /stats/providers` return real rollup data instead of an empty array.

**Files:**
- Modify: `crates/vaultd/src/routes/stats.rs`

- [ ] **Step 1: Update the stats route**

File: `crates/vaultd/src/routes/stats.rs`

```rust
use axum::{extract::State, Json};
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

pub async fn stats_providers(State(state): State<AppState>) -> Json<serde_json::Value> {
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
```

- [ ] **Step 2: Verify compilation**

Run: `cargo build -p vaultd`

Expected: compiles.

- [ ] **Step 3: Commit**

Run: `git add crates/vaultd && git commit -m "feat: wire real stats into vaultd stats route"`

---

## Final verification checklist

- `cargo fmt --all`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test`
- Manual: start vaultd, verify `GET /health` and `GET /stats/providers` work
- Manual: create a proxy binding, issue a lease, test `POST /v1/proxy/openai/v1/chat/completions` (requires a real API key)

## Recommended implementation order

**Execute the telemetry plan (step5-telemetry-and-stats.md) first**, then execute tasks 1-5 of this plan in sequence.
