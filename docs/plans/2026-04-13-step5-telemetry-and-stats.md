# Telemetry & Stats Implementation Plan

> **For agentic workers:** Use subagent-driven-development to implement this plan task-by-task.

**Goal:** Record usage events in SQLite and expose rollup statistics via `vault stats` CLI and `GET /stats/providers` HTTP endpoint.

**Architecture:** Add a `usage_events` repository module to `vault-db`, implement the existing `TelemetryWriter` stub in `vault-telemetry`, add rollup queries that aggregate by provider/agent/time, and wire the `vault stats` CLI command and the vaultd stats route to real data.

**Tech Stack:** Rust 2024, sqlx + SQLite, tokio, clap, axum.

**Out of scope:**
- Real-time streaming stats
- Cost estimation logic (use placeholder from `ParsedUsage`)
- Proxy-originated events (covered by the proxy plan)

---

## Shared conventions for every task

- Run `cargo fmt --all` after each task that changes Rust files
- Run the narrowest possible test first, then `cargo test`
- Database URL for local development: `sqlite:.local/vault.db`

---

### Task 1: Add usage_events repository to vault-db

**Objective:** Provide insert and query methods for usage events so both telemetry writer and stats can use them.

**Files:**
- Create: `crates/vault-db/src/usage_events.rs`
- Modify: `crates/vault-db/src/lib.rs`
- Test: `crates/vault-db/tests/repositories.rs`

- [ ] **Step 1: Write a failing test**

File: `crates/vault-db/tests/repositories.rs`

Add at the end:

```rust
#[tokio::test]
async fn insert_usage_event_should_be_queryable() {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite:{}", dir.path().join("test.db").display());
    let store = Store::connect(&url).await.unwrap();

    let credential = vault_core::models::Credential {
        id: uuid::Uuid::new_v4(),
        provider: "openai".to_string(),
        kind: vault_core::models::CredentialKind::ApiKey,
        label: "work".to_string(),
        secret_ref: "ref".to_string(),
        environment: "work".to_string(),
        owner: None,
        enabled: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_used_at: None,
    };
    store.insert_credential(&credential).await.unwrap();

    let event = vault_core::models::UsageEvent {
        id: uuid::Uuid::new_v4(),
        provider: "openai".to_string(),
        credential_id: credential.id,
        lease_id: None,
        agent_name: "codex".to_string(),
        project: Some("my-project".to_string()),
        mode: vault_core::models::AccessMode::Inject,
        operation: "process_launch".to_string(),
        endpoint: None,
        model: None,
        request_count: 1,
        prompt_tokens: None,
        completion_tokens: None,
        total_tokens: None,
        estimated_cost_usd: None,
        status_code: None,
        success: true,
        latency_ms: 42,
        error_text: None,
        created_at: chrono::Utc::now(),
    };
    store.insert_usage_event(&event).await.unwrap();

    let events = store.list_usage_events(10).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].agent_name, "codex");
    assert_eq!(events[0].provider, "openai");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vault-db insert_usage_event_should_be_queryable -- --nocapture`

Expected: fail because `insert_usage_event` and `list_usage_events` do not exist.

- [ ] **Step 3: Implement the usage_events repository**

File: `crates/vault-db/src/usage_events.rs`

```rust
use anyhow::{Context, Result};
use vault_core::models::{AccessMode, UsageEvent};

use crate::Store;
use crate::codec::{access_mode_from_str, access_mode_to_str};

impl Store {
    pub async fn insert_usage_event(&self, event: &UsageEvent) -> Result<()> {
        let id = event.id.to_string();
        let credential_id = event.credential_id.to_string();
        let lease_id = event.lease_id.map(|lid| lid.to_string());
        let mode = access_mode_to_str(&event.mode);
        let created_at = event.created_at.to_rfc3339();
        let success_int: i32 = if event.success { 1 } else { 0 };

        sqlx::query(
            "INSERT INTO usage_events \
             (id, provider, credential_id, lease_id, agent_name, project, mode, operation, \
              endpoint, model, request_count, prompt_tokens, completion_tokens, total_tokens, \
              estimated_cost_usd, status_code, success, latency_ms, error_text, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&event.provider)
        .bind(&credential_id)
        .bind(&lease_id)
        .bind(&event.agent_name)
        .bind(&event.project)
        .bind(&mode)
        .bind(&event.operation)
        .bind(&event.endpoint)
        .bind(&event.model)
        .bind(event.request_count)
        .bind(event.prompt_tokens)
        .bind(event.completion_tokens)
        .bind(event.total_tokens)
        .bind(event.estimated_cost_usd)
        .bind(event.status_code)
        .bind(success_int)
        .bind(event.latency_ms)
        .bind(&event.error_text)
        .bind(&created_at)
        .execute(&self.pool)
        .await
        .context("failed to insert usage event")?;

        Ok(())
    }

    pub async fn list_usage_events(&self, limit: i64) -> Result<Vec<UsageEvent>> {
        let rows = sqlx::query_as::<_, UsageEventRow>(
            "SELECT id, provider, credential_id, lease_id, agent_name, project, mode, \
             operation, endpoint, model, request_count, prompt_tokens, completion_tokens, \
             total_tokens, estimated_cost_usd, status_code, success, latency_ms, error_text, \
             created_at \
             FROM usage_events ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("failed to list usage events")?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }
}

#[derive(sqlx::FromRow)]
struct UsageEventRow {
    id: String,
    provider: String,
    credential_id: String,
    lease_id: Option<String>,
    agent_name: String,
    project: Option<String>,
    mode: String,
    operation: String,
    endpoint: Option<String>,
    model: Option<String>,
    request_count: i64,
    prompt_tokens: Option<i64>,
    completion_tokens: Option<i64>,
    total_tokens: Option<i64>,
    estimated_cost_usd: Option<f64>,
    status_code: Option<i64>,
    success: i64,
    latency_ms: i64,
    error_text: Option<String>,
    created_at: String,
}

impl TryFrom<UsageEventRow> for UsageEvent {
    type Error = anyhow::Error;

    fn try_from(row: UsageEventRow) -> Result<Self> {
        Ok(UsageEvent {
            id: uuid::Uuid::parse_str(&row.id)?,
            provider: row.provider,
            credential_id: uuid::Uuid::parse_str(&row.credential_id)?,
            lease_id: row.lease_id.map(|s| uuid::Uuid::parse_str(&s)).transpose()?,
            agent_name: row.agent_name,
            project: row.project,
            mode: access_mode_from_str(&row.mode)?,
            operation: row.operation,
            endpoint: row.endpoint,
            model: row.model,
            request_count: row.request_count,
            prompt_tokens: row.prompt_tokens,
            completion_tokens: row.completion_tokens,
            total_tokens: row.total_tokens,
            estimated_cost_usd: row.estimated_cost_usd,
            status_code: row.status_code,
            success: row.success != 0,
            latency_ms: row.latency_ms,
            error_text: row.error_text,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)?.with_timezone(&chrono::Utc),
        })
    }
}
```

Add to `crates/vault-db/src/lib.rs`:

```rust
mod usage_events;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vault-db insert_usage_event_should_be_queryable -- --nocapture`

Expected: pass.

- [ ] **Step 5: Commit**

Run: `git add crates/vault-db && git commit -m "feat: add usage events repository"`

---

### Task 2: Add rollup query to vault-db

**Objective:** Provide an aggregation query that summarizes usage events by provider, suitable for the stats command.

**Files:**
- Modify: `crates/vault-db/src/usage_events.rs`
- Test: `crates/vault-db/tests/repositories.rs`

- [ ] **Step 1: Write a failing test**

File: `crates/vault-db/tests/repositories.rs`

Add:

```rust
#[tokio::test]
async fn usage_stats_by_provider_should_aggregate() {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite:{}", dir.path().join("test.db").display());
    let store = Store::connect(&url).await.unwrap();

    let credential = vault_core::models::Credential {
        id: uuid::Uuid::new_v4(),
        provider: "openai".to_string(),
        kind: vault_core::models::CredentialKind::ApiKey,
        label: "work".to_string(),
        secret_ref: "ref".to_string(),
        environment: "work".to_string(),
        owner: None,
        enabled: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_used_at: None,
    };
    store.insert_credential(&credential).await.unwrap();

    for i in 0..3 {
        let event = vault_core::models::UsageEvent {
            id: uuid::Uuid::new_v4(),
            provider: "openai".to_string(),
            credential_id: credential.id,
            lease_id: None,
            agent_name: "codex".to_string(),
            project: None,
            mode: vault_core::models::AccessMode::Inject,
            operation: "process_launch".to_string(),
            endpoint: None,
            model: None,
            request_count: 1,
            prompt_tokens: Some(100),
            completion_tokens: Some(50),
            total_tokens: Some(150),
            estimated_cost_usd: Some(0.01),
            status_code: None,
            success: true,
            latency_ms: 10 + i,
            error_text: None,
            created_at: chrono::Utc::now(),
        };
        store.insert_usage_event(&event).await.unwrap();
    }

    let stats = store.usage_stats_by_provider().await.unwrap();
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].provider, "openai");
    assert_eq!(stats[0].request_count, 3);
    assert_eq!(stats[0].prompt_tokens, 300);
    assert_eq!(stats[0].completion_tokens, 150);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vault-db usage_stats_by_provider -- --nocapture`

Expected: fail.

- [ ] **Step 3: Implement the rollup query**

Add to `crates/vault-db/src/usage_events.rs`:

```rust
#[derive(Debug, Clone)]
pub struct ProviderStats {
    pub provider: String,
    pub request_count: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub estimated_cost_usd: f64,
    pub last_used_at: String,
}

#[derive(sqlx::FromRow)]
struct ProviderStatsRow {
    provider: String,
    request_count: i64,
    prompt_tokens: i64,
    completion_tokens: i64,
    total_tokens: i64,
    estimated_cost_usd: f64,
    last_used_at: String,
}

impl Store {
    pub async fn usage_stats_by_provider(&self) -> Result<Vec<ProviderStats>> {
        let rows = sqlx::query_as::<_, ProviderStatsRow>(
            "SELECT provider, \
             SUM(request_count) as request_count, \
             COALESCE(SUM(prompt_tokens), 0) as prompt_tokens, \
             COALESCE(SUM(completion_tokens), 0) as completion_tokens, \
             COALESCE(SUM(total_tokens), 0) as total_tokens, \
             COALESCE(SUM(estimated_cost_usd), 0.0) as estimated_cost_usd, \
             MAX(created_at) as last_used_at \
             FROM usage_events \
             GROUP BY provider \
             ORDER BY request_count DESC",
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to query usage stats by provider")?;

        Ok(rows
            .into_iter()
            .map(|r| ProviderStats {
                provider: r.provider,
                request_count: r.request_count,
                prompt_tokens: r.prompt_tokens,
                completion_tokens: r.completion_tokens,
                total_tokens: r.total_tokens,
                estimated_cost_usd: r.estimated_cost_usd,
                last_used_at: r.last_used_at,
            })
            .collect())
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vault-db usage_stats_by_provider -- --nocapture`

Expected: pass.

- [ ] **Step 5: Commit**

Run: `git add crates/vault-db && git commit -m "feat: add usage stats rollup query"`

---

### Task 3: Implement TelemetryWriter

**Objective:** Replace the no-op stub with a real implementation that persists usage events.

**Files:**
- Modify: `crates/vault-telemetry/src/writer.rs`
- Modify: `crates/vault-telemetry/Cargo.toml`
- Test: `crates/vault-telemetry/src/writer.rs`

- [ ] **Step 1: Write a failing test**

File: `crates/vault-telemetry/src/writer.rs`

Add at the end:

```rust
#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;
    use vault_core::models::{AccessMode, Credential, CredentialKind, UsageEvent};
    use vault_db::Store;

    use super::TelemetryWriter;

    #[tokio::test]
    async fn write_usage_event_should_persist_to_db() {
        let dir = tempfile::tempdir().unwrap();
        let url = format!("sqlite:{}", dir.path().join("test.db").display());
        let store = Store::connect(&url).await.unwrap();

        let credential = Credential {
            id: Uuid::new_v4(),
            provider: "openai".to_string(),
            kind: CredentialKind::ApiKey,
            label: "test".to_string(),
            secret_ref: "ref".to_string(),
            environment: "work".to_string(),
            owner: None,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_used_at: None,
        };
        store.insert_credential(&credential).await.unwrap();

        let writer = TelemetryWriter::new(store.clone());
        let event = UsageEvent {
            id: Uuid::new_v4(),
            provider: "openai".to_string(),
            credential_id: credential.id,
            lease_id: None,
            agent_name: "test-agent".to_string(),
            project: None,
            mode: AccessMode::Inject,
            operation: "process_launch".to_string(),
            endpoint: None,
            model: None,
            request_count: 1,
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
            estimated_cost_usd: None,
            status_code: None,
            success: true,
            latency_ms: 100,
            error_text: None,
            created_at: Utc::now(),
        };

        writer.write_usage_event(&event).await.unwrap();

        let events = store.list_usage_events(10).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].agent_name, "test-agent");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vault-telemetry write_usage_event_should_persist -- --nocapture`

Expected: fail because the body is a no-op stub.

- [ ] **Step 3: Implement the writer**

File: `crates/vault-telemetry/src/writer.rs`

Replace the stub body:

```rust
pub async fn write_usage_event(&self, event: &UsageEvent) -> anyhow::Result<()> {
    self.store.insert_usage_event(event).await
}
```

Add `tempfile = "3"` to `[dev-dependencies]` in `crates/vault-telemetry/Cargo.toml`, and add `tokio.workspace = true` to `[dependencies]`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vault-telemetry -- --nocapture`

Expected: pass.

- [ ] **Step 5: Commit**

Run: `git add crates/vault-telemetry && git commit -m "feat: implement telemetry writer"`

---

### Task 4: Wire vault stats CLI command

**Objective:** Make `vault stats` print real rollup data from the database.

**Files:**
- Modify: `crates/vault-cli/src/commands/stats.rs`
- Test: `crates/vault-cli/src/commands/stats.rs`

- [ ] **Step 1: Write a failing test**

File: `crates/vault-cli/src/commands/stats.rs`

Replace the entire file:

```rust
use anyhow::Result;
use clap::Args;
use vault_db::Store;

use crate::support::config::current_database_url;

#[derive(Debug, Args)]
pub struct StatsCommand {
    #[arg(long)]
    pub provider: Option<String>,
}

pub async fn run_stats_command(cmd: StatsCommand) -> Result<()> {
    let store = Store::connect(&current_database_url()).await?;
    let stats = store.usage_stats_by_provider().await?;

    if stats.is_empty() {
        println!("No usage events recorded yet.");
        return Ok(());
    }

    for stat in &stats {
        if let Some(ref filter) = cmd.provider {
            if stat.provider != *filter {
                continue;
            }
        }
        println!(
            "provider={} requests={} prompt_tokens={} completion_tokens={} cost_usd={:.4} last_used={}",
            stat.provider,
            stat.request_count,
            stat.prompt_tokens,
            stat.completion_tokens,
            stat.estimated_cost_usd,
            stat.last_used_at,
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;
    use vault_core::models::{AccessMode, Credential, CredentialKind, UsageEvent};
    use vault_db::Store;

    use crate::support::config::{
        clear_test_database_url, current_database_url, set_test_database_url, test_database_lock,
    };

    use super::run_stats_command;
    use super::StatsCommand;

    fn setup_test_db() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_url = format!("sqlite:{}", dir.path().join("stats.db").display());
        set_test_database_url(db_url);
        dir
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn stats_should_show_empty_message_when_no_events() {
        let _guard = test_database_lock().lock().expect("test lock");
        let _dir = setup_test_db();

        let result = run_stats_command(StatsCommand { provider: None }).await;
        clear_test_database_url();
        result.expect("stats command should succeed");
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn stats_should_show_provider_summary() {
        let _guard = test_database_lock().lock().expect("test lock");
        let _dir = setup_test_db();

        let store = Store::connect(&current_database_url()).await.expect("connect");
        let credential = Credential {
            id: Uuid::new_v4(),
            provider: "openai".to_string(),
            kind: CredentialKind::ApiKey,
            label: "test".to_string(),
            secret_ref: "ref".to_string(),
            environment: "work".to_string(),
            owner: None,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_used_at: None,
        };
        store.insert_credential(&credential).await.expect("insert");

        let event = UsageEvent {
            id: Uuid::new_v4(),
            provider: "openai".to_string(),
            credential_id: credential.id,
            lease_id: None,
            agent_name: "test".to_string(),
            project: None,
            mode: AccessMode::Inject,
            operation: "launch".to_string(),
            endpoint: None,
            model: None,
            request_count: 1,
            prompt_tokens: Some(100),
            completion_tokens: Some(50),
            total_tokens: Some(150),
            estimated_cost_usd: Some(0.005),
            status_code: None,
            success: true,
            latency_ms: 10,
            error_text: None,
            created_at: Utc::now(),
        };
        store.insert_usage_event(&event).await.expect("insert event");

        let result = run_stats_command(StatsCommand { provider: None }).await;
        clear_test_database_url();
        result.expect("stats command should succeed");
    }
}
```

- [ ] **Step 2: Run test to verify it passes** (this is a replacement, not TDD — the command was a hollow stub)

Run: `cargo test -p vault-cli stats -- --nocapture`

Expected: pass.

- [ ] **Step 3: Update the stats subcommand wiring in main.rs if needed**

Check `crates/vault-cli/src/main.rs` and ensure `StatsCommand` is wired with the updated struct (remove `profile` field if it was there).

- [ ] **Step 4: Run full test suite**

Run: `cargo test`

Expected: all pass.

- [ ] **Step 5: Commit**

Run: `git add crates/vault-cli && git commit -m "feat: wire vault stats command to real rollup data"`

---

### Task 5: Record launch events in vault run

**Objective:** When `vault run` finishes, record a usage event so stats reflect CLI usage even before proxy is implemented.

**Files:**
- Modify: `crates/vault-cli/src/commands/run.rs`
- Modify: `crates/vault-cli/Cargo.toml` (add vault-telemetry dep if not present)

- [ ] **Step 1: Add vault-telemetry dependency**

Add to `crates/vault-cli/Cargo.toml` under `[dependencies]`:

```toml
vault-telemetry = { path = "../vault-telemetry" }
```

- [ ] **Step 2: Implement event recording after child process exits**

In `crates/vault-cli/src/commands/run.rs`, after the child process `status` is obtained and before the success/failure check, add:

```rust
// Record a launch usage event for audit and stats.
{
    let telemetry = vault_telemetry::TelemetryWriter::new(store.clone());
    let launch_event = vault_core::models::UsageEvent {
        id: uuid::Uuid::new_v4(),
        provider: "vault".to_string(),
        credential_id: uuid::Uuid::nil(),
        lease_id: Some(lease.id),
        agent_name: cmd.agent.clone(),
        project: cmd.project.clone(),
        mode: vault_core::models::AccessMode::Inject,
        operation: "process_launch".to_string(),
        endpoint: None,
        model: None,
        request_count: 1,
        prompt_tokens: None,
        completion_tokens: None,
        total_tokens: None,
        estimated_cost_usd: None,
        status_code: status.code().map(|c| c as i64),
        success: status.success(),
        latency_ms: 0,
        error_text: if status.success() { None } else { Some(format!("exit {status}")) },
        created_at: chrono::Utc::now(),
    };
    if let Err(err) = telemetry.write_usage_event(&launch_event).await {
        debug_log(format!("failed to record launch event: {err}"));
    }
}
```

Note: `credential_id` uses `Uuid::nil()` because the launch event is a meta-event, not tied to a single credential. The `store` variable needs to remain in scope — move its binding before the child spawn if necessary.

- [ ] **Step 3: Run tests and manual verification**

Run: `cargo test -p vault-cli -- --nocapture`

Expected: pass.

- [ ] **Step 4: Commit**

Run: `git add crates/vault-cli && git commit -m "feat: record launch audit events in vault run"`

---

## Final verification checklist

- `cargo fmt --all`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test`

## Recommended implementation order

Tasks 1 through 5 in sequence — each depends on the previous.
