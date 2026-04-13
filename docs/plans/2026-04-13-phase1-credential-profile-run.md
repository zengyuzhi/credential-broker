# Credential/Profile/Run Phase 1 Implementation Plan

> For Hermes: Use subagent-driven-development to implement this plan task-by-task.

Goal: Implement the first usable slice of credential-broker so it can securely store credentials, create profiles, bind providers to credentials, and launch agent subprocesses with injected environment variables.

Architecture: This phase stays entirely in local CLI mode. Secrets are stored in macOS Keychain, metadata lives in SQLite, policy checks happen in-process, and `vault run` resolves bound credentials into child-process environment variables. The daemon and HTTP proxy remain out of scope for this phase except for keeping interfaces compatible with later proxy work.

Tech Stack: Rust, clap, sqlx + SQLite, security-framework, tokio, tracing.

Out of scope for this phase:
- HTTP proxy routes
- Full usage request accounting
- OAuth refresh flows
- Team/multi-user features

---

## Phase 1 acceptance criteria

By the end of this phase, all of the following should work:
- `vault credential add <provider> <label> --kind api_key --env work`
- `vault credential list`
- `vault credential disable <id>` / `enable <id>` / `remove <id>`
- `vault profile create <name>`
- `vault profile list`
- `vault profile bind <profile> <provider> <credential-id> --mode inject|either`
- `vault profile show <name>`
- `vault run --profile coding --agent codex -- <command...>`
- injected env vars match provider bindings
- secrets never appear in stdout, db rows, or logs

---

## Shared conventions for every coding task

- Database URL for local development: `sqlite:.local/vault.db`
- Keychain service name: `ai.zyr1.vault`
- Use tests before implementation when behavior changes
- Run `cargo fmt --all` after each task that changes Rust files
- Run the narrowest possible test first, then `cargo test`

---

### Task 1: Add a development harness and database bootstrap

Objective: Make Phase 1 development repeatable by adding helpers for opening the SQLite database and ensuring migrations can be applied from the CLI.

Files:
- Modify: `crates/vault-db/src/lib.rs`
- Modify: `crates/vault-db/src/store.rs`
- Create: `crates/vault-db/tests/store_smoke.rs`
- Create: `.local/.gitkeep`

Step 1: Write a failing smoke test

File: `crates/vault-db/tests/store_smoke.rs`

Test shape:
```rust
#[tokio::test]
async fn connect_should_open_sqlite_database() {
    let dir = tempfile::tempdir().unwrap();
    let db_url = format!("sqlite:{}", dir.path().join("test.db").display());
    let store = Store::connect(&db_url).await.unwrap();
    let row: (i64,) = sqlx::query_as("select 1").fetch_one(&store.pool).await.unwrap();
    assert_eq!(row.0, 1);
}
```

Step 2: Run the test and confirm failure

Run:
`cargo test -p vault-db connect_should_open_sqlite_database -- --nocapture`

Expected: fail because `tempfile` is not yet available or helper wiring is incomplete.

Step 3: Add missing test dependency and connection helper

Implementation details:
- add `tempfile = "3"` under `[dev-dependencies]` in `crates/vault-db/Cargo.toml`
- keep `Store::connect()` as the canonical constructor
- export `Store` from `crates/vault-db/src/lib.rs`

Step 4: Re-run the test

Run:
`cargo test -p vault-db connect_should_open_sqlite_database -- --nocapture`

Expected: pass.

Step 5: Commit

Run:
`git add crates/vault-db .local/.gitkeep && git commit -m "feat: add db bootstrap smoke test"`

---

### Task 2: Add repository methods for credentials and profiles

Objective: Replace the raw `Store` shell with repository methods used by the CLI commands.

Files:
- Modify: `crates/vault-db/src/store.rs`
- Create: `crates/vault-db/src/credentials.rs`
- Create: `crates/vault-db/src/profiles.rs`
- Create: `crates/vault-db/src/bindings.rs`
- Modify: `crates/vault-db/src/lib.rs`
- Test: `crates/vault-db/tests/repositories.rs`

Step 1: Write a failing repository test

Tests to add:
- insert credential metadata
- fetch credentials ordered by provider/label
- create profile
- bind provider to profile
- fetch profile with bindings

Example test case:
```rust
#[tokio::test]
async fn insert_credential_should_be_listed() {
    // setup temp db and schema
    // insert a Credential
    // list credentials
    // assert one result with expected provider+label
}
```

Step 2: Run test to confirm failure

Run:
`cargo test -p vault-db insert_credential_should_be_listed -- --nocapture`

Expected: fail because repository methods do not exist.

Step 3: Implement repository methods

Minimum methods:
- `insert_credential(&Credential)`
- `list_credentials() -> Vec<Credential>`
- `set_credential_enabled(id, enabled)`
- `delete_credential(id)`
- `insert_profile(&Profile)`
- `list_profiles() -> Vec<Profile>`
- `insert_binding(&ProfileBinding)`
- `get_profile_by_name(name)`
- `list_bindings_for_profile(profile_id)`

Step 4: Re-run focused tests, then full crate tests

Run:
`cargo test -p vault-db -- --nocapture`

Expected: pass.

Step 5: Commit

Run:
`git add crates/vault-db && git commit -m "feat: add db repositories for credentials and profiles"`

---

### Task 3: Define provider field schemas for Phase 1

Objective: Make provider support explicit so CLI validation and env injection do not hardcode string logic in multiple places.

Files:
- Modify: `crates/vault-providers/src/registry.rs`
- Create: `crates/vault-providers/src/schema.rs`
- Modify: `crates/vault-providers/src/lib.rs`
- Test: `crates/vault-providers/src/registry.rs`

Step 1: Write a failing provider-schema test

Add tests that verify:
- `openai` requires `api_key`
- `anthropic` requires `api_key`
- `openrouter` requires `api_key`
- `twitterapi` requires `api_key`
- `github` requires `token`
- `tavily` requires `api_key`
- `coingecko` requires `api_key`

Step 2: Run the test

Run:
`cargo test -p vault-providers provider_schema -- --nocapture`

Expected: fail because registry only supports 3 providers now.

Step 3: Implement schema metadata

Add a struct like:
```rust
pub struct ProviderSchema {
    pub provider: &'static str,
    pub required_fields: &'static [&'static str],
    pub default_mode: AccessMode,
}
```

Add helper:
- `schema_for(provider: &str) -> Option<ProviderSchema>`

Phase 1 must support all seven providers at schema level even if only three have proxy adapters.

Step 4: Re-run tests

Run:
`cargo test -p vault-providers -- --nocapture`

Expected: pass.

Step 5: Commit

Run:
`git add crates/vault-providers && git commit -m "feat: add provider field schemas for phase 1"`

---

### Task 4: Implement hidden secret input and keychain persistence

Objective: Turn `vault credential add` into a real command that prompts securely and stores secrets in Keychain.

Files:
- Modify: `crates/vault-cli/Cargo.toml`
- Modify: `crates/vault-cli/src/commands/credential.rs`
- Modify: `crates/vault-secrets/src/lib.rs`
- Modify: `crates/vault-secrets/src/keychain.rs`
- Create: `crates/vault-cli/src/support/prompt.rs`
- Create: `crates/vault-cli/src/support/mod.rs`

Step 1: Write a failing parsing/unit test first

Test target:
- provider schema returns required fields
- generated keychain account refs are stable

Example test:
```rust
#[test]
fn keychain_account_name_should_include_credential_id_and_field() {
    let account = build_keychain_account("123", "api_key");
    assert_eq!(account, "credential:123:api_key");
}
```

Step 2: Run the test

Run:
`cargo test -p vault-cli keychain_account_name_should_include_credential_id_and_field -- --nocapture`

Expected: fail because helper does not exist.

Step 3: Implement command flow

Behavior of `vault credential add`:
- validate provider via `schema_for()`
- create UUID
- prompt for required secret fields without echo
- store each secret field in Keychain under `ai.zyr1.vault` / `credential:<uuid>:<field>`
- insert metadata row into SQLite
- print only id/provider/label/environment

Suggested crate additions:
- add `rpassword = "7"` to CLI dependencies
- add `uuid`, `chrono`, `vault-db`, `vault-secrets`, `vault-core`, `vault-providers` to `vault-cli/Cargo.toml`

Step 4: Manual verification

Run:
`cargo run -p vault-cli -- credential add twitterapi social-main --kind api_key --env work`

Expected:
- prompt asks for `api_key`
- input is hidden
- success output shows credential id only

Step 5: Commit

Run:
`git add crates/vault-cli crates/vault-secrets && git commit -m "feat: add secure credential creation flow"`

---

### Task 5: Implement credential list/enable/disable/remove

Objective: Finish the credential lifecycle so users can manage stored entries safely.

Files:
- Modify: `crates/vault-cli/src/commands/credential.rs`
- Modify: `crates/vault-db/src/credentials.rs`
- Test: `crates/vault-db/tests/repositories.rs`
- Test: `crates/vault-cli/src/commands/credential.rs`

Step 1: Write failing tests

Add tests for:
- disabling a credential marks `enabled = false`
- enabling flips it back
- removing deletes SQLite metadata
- removing also calls Keychain delete for each field reference

Step 2: Run focused tests

Run:
`cargo test -p vault-db set_credential_enabled -- --nocapture`

Expected: fail until methods are implemented.

Step 3: Implement command handlers

Behavior:
- `list` prints table: id / provider / label / env / enabled / last_used_at
- `disable` and `enable` act only on existing ids
- `remove` confirms via `--yes` flag or interactive prompt before deleting secrets

If you want to avoid interactive confirmations in Phase 1, require `--yes` and fail otherwise.

Step 4: Manual verification

Run:
- `cargo run -p vault-cli -- credential list`
- `cargo run -p vault-cli -- credential disable <id>`
- `cargo run -p vault-cli -- credential enable <id>`

Expected: state changes are visible in list output.

Step 5: Commit

Run:
`git add crates/vault-cli crates/vault-db && git commit -m "feat: add credential lifecycle commands"`

---

### Task 6: Implement profile create/list/show

Objective: Make named profiles real so agent execution can target a stable bundle of provider bindings.

Files:
- Modify: `crates/vault-cli/src/commands/profile.rs`
- Modify: `crates/vault-db/src/profiles.rs`
- Test: `crates/vault-db/tests/repositories.rs`
- Test: `crates/vault-cli/src/commands/profile.rs`

Step 1: Write failing tests

Add tests for:
- create profile persists a row
- list profiles returns created profiles
- show profile returns profile even with no bindings yet

Step 2: Run test

Run:
`cargo test -p vault-db create_profile -- --nocapture`

Expected: fail until repository methods are wired.

Step 3: Implement command behavior

Commands:
- `vault profile create coding`
- `vault profile list`
- `vault profile show coding`

Show output should include:
- profile id
- name
- description if present
- default project if present
- zero or more bindings

Step 4: Manual verification

Run:
- `cargo run -p vault-cli -- profile create coding`
- `cargo run -p vault-cli -- profile list`
- `cargo run -p vault-cli -- profile show coding`

Expected: `coding` exists and shows no bindings initially.

Step 5: Commit

Run:
`git add crates/vault-cli crates/vault-db && git commit -m "feat: add profile commands"`

---

### Task 7: Implement profile binding with policy checks

Objective: Allow profiles to map providers to credentials while rejecting invalid combinations.

Files:
- Modify: `crates/vault-cli/src/commands/profile.rs`
- Modify: `crates/vault-policy/src/service.rs`
- Modify: `crates/vault-db/src/bindings.rs`
- Test: `crates/vault-policy/src/service.rs`
- Test: `crates/vault-db/tests/repositories.rs`

Step 1: Write failing tests

Cases to cover:
- binding fails if credential is disabled
- binding fails if environment is `prod` and `allow_prod = false`
- binding fails if mode is unsupported for the provider
- binding upserts by `(profile, provider)` rather than creating duplicates

Step 2: Run focused tests

Run:
`cargo test -p vault-policy -- --nocapture`

Expected: fail until checks are implemented.

Step 3: Implement policy flow

Before inserting binding:
- load profile
- load credential
- validate provider matches binding provider
- validate credential enabled
- validate environment via `PolicyService`
- validate mode against provider adapter/schema

For Phase 1, allow only `inject` and `either` on the CLI. Keep `proxy` reserved for Phase 2, but still parse it if you want future compatibility.

Step 4: Manual verification

Run:
`cargo run -p vault-cli -- profile bind coding twitterapi <credential-id> --mode inject`

Then:
`cargo run -p vault-cli -- profile show coding`

Expected: binding appears under coding.

Step 5: Commit

Run:
`git add crates/vault-cli crates/vault-policy crates/vault-db && git commit -m "feat: add profile binding with policy validation"`

---

### Task 8: Wire leases into CLI execution

Objective: Make `vault run` issue a short-lived lease before launching the child process so the audit model is already in place for later proxy support.

Files:
- Modify: `crates/vault-cli/src/commands/run.rs`
- Modify: `crates/vault-policy/src/lease.rs`
- Modify: `crates/vault-db/src/store.rs`
- Test: `crates/vault-policy/src/lease.rs`

Step 1: Write failing tests

Add tests for:
- `issue_lease()` returns a raw token and stored hash
- lease expiration is later than issue time

Step 2: Run tests

Run:
`cargo test -p vault-policy token_hash_is_stable -- --nocapture`

Expected: add one more failing test for `issue_lease` behavior first, then pass after implementation.

Step 3: Implement run-time lease issuance

Flow in `vault run`:
- load profile by name
- issue a lease using `issue_lease(profile_id, agent, project, ttl)`
- persist lease in SQLite
- keep raw token only in memory for this process

For Phase 1 the child process does not need the lease token yet unless you decide to inject `VAULT_LEASE_TOKEN` for future compatibility.

Step 4: Verify

Run:
`cargo test -p vault-policy -- --nocapture`

Expected: pass.

Step 5: Commit

Run:
`git add crates/vault-cli crates/vault-policy crates/vault-db && git commit -m "feat: issue leases for vault run"`

---

### Task 9: Implement env resolution and child-process launch

Objective: Make `vault run` actually usable for Codex/Hermes by resolving bindings into environment variables and spawning a command.

Files:
- Modify: `crates/vault-cli/src/commands/run.rs`
- Modify: `crates/vault-providers/src/registry.rs`
- Modify: `crates/vault-db/src/bindings.rs`
- Possibly create: `crates/vault-cli/src/support/process.rs`
- Test: `crates/vault-cli/tests/run_env.rs`

Step 1: Write a failing integration test

Test idea:
- create temp db
- create one profile bound to an openai credential
- simulate resolved secret values using a fake secret store
- build child env map
- assert `OPENAI_API_KEY` is present

If full process-spawn integration is too heavy for the first pass, split into:
- `resolve_env_for_profile()` unit test
- process-launch smoke test separately

Step 2: Run focused test

Run:
`cargo test -p vault-cli resolve_env_for_profile -- --nocapture`

Expected: fail because resolver does not exist.

Step 3: Implement resolver + launch

Required behavior:
- load all bindings for profile
- for each binding in inject/either mode, resolve secret fields from Keychain
- ask provider adapter for env map
- merge env vars into child process
- preserve parent env vars unless explicitly overwritten
- optionally inject:
  - `VAULT_PROFILE=<profile>`
  - `VAULT_AGENT=<agent>`
  - `VAULT_LEASE_TOKEN=<token>`

Launch with `tokio::process::Command`.

Step 4: Manual verification

Run:
`cargo run -p vault-cli -- run --profile coding --agent demo -- env | grep -E 'OPENAI|ANTHROPIC|TWITTER|GITHUB|VAULT_'`

Expected: only variables for the profile bindings appear.

Step 5: Commit

Run:
`git add crates/vault-cli crates/vault-providers crates/vault-db && git commit -m "feat: add env resolution and command execution"`

---

### Task 10: Record launch audit events and add a simple stats read path

Objective: Even before full HTTP proxy telemetry, capture process-launch usage so the ledger starts paying off immediately.

Files:
- Modify: `crates/vault-telemetry/src/writer.rs`
- Modify: `crates/vault-cli/src/commands/run.rs`
- Modify: `crates/vault-cli/src/commands/stats.rs`
- Test: `crates/vault-telemetry/src/writer.rs`

Step 1: Write failing test

Add a test for `record_launch_event()` that inserts one `UsageEvent` with:
- provider = `vault`
- operation = `process_launch`
- agent_name = input agent
- project = input project
- success = true/false based on child exit status

Step 2: Run test

Run:
`cargo test -p vault-telemetry -- --nocapture`

Expected: fail because writer is a stub.

Step 3: Implement writer and simple stats

Phase 1 stats may be minimal:
- total launch count
- launches by agent
- most recent run timestamp

`vault stats` can start by querying `usage_events` directly.

Step 4: Manual verification

Run:
- `cargo run -p vault-cli -- run --profile coding --agent demo -- true`
- `cargo run -p vault-cli -- stats`

Expected: stats output reflects at least one launch event.

Step 5: Commit

Run:
`git add crates/vault-telemetry crates/vault-cli && git commit -m "feat: record launch audit events and basic stats"`

---

### Task 11: Add redaction and safety checks before using real secrets

Objective: Ensure Phase 1 never leaks secrets while printing command results or credential metadata.

Files:
- Create: `crates/vault-core/src/redaction.rs`
- Modify: `crates/vault-core/src/lib.rs`
- Modify: `crates/vault-cli/src/commands/credential.rs`
- Modify: `crates/vault-cli/src/commands/run.rs`
- Test: `crates/vault-core/src/redaction.rs`

Step 1: Write failing tests

Cases:
- redacts strings that look like API keys when formatting output
- credential list never prints secret refs as secret values
- `vault run` debug output never prints injected env values

Step 2: Run tests

Run:
`cargo test -p vault-core -- --nocapture`

Expected: fail until redaction helpers exist.

Step 3: Implement minimal redaction

Functions:
- `redact_secret(value: &str) -> String`
- `redact_env_map(map: &HashMap<String, String>) -> HashMap<String, String>`

Behavior:
- keep last 4 chars only if length >= 8
- otherwise replace entire secret with `***`

Step 4: Re-run full suite

Run:
`cargo fmt --all && cargo test`

Expected: pass.

Step 5: Commit

Run:
`git add crates/vault-core crates/vault-cli && git commit -m "feat: add redaction safeguards for phase 1"`

---

## Final verification checklist for Phase 1

Run these commands in order:
- `cargo fmt --all`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test`
- `cargo run -p vault-cli -- credential list`
- `cargo run -p vault-cli -- profile list`

Manual end-to-end check:
1. Add one `openai` credential
2. Add one `twitterapi` credential
3. Create profile `coding`
4. Bind `openai` to `coding`
5. Bind `twitterapi` to `coding`
6. Run:
   `cargo run -p vault-cli -- run --profile coding --agent demo -- env | grep -E 'OPENAI|TWITTER'`
7. Confirm env names appear, but secret values are only present in the child process and not printed by the CLI
8. Run `cargo run -p vault-cli -- stats`
9. Confirm at least one launch event exists

## Recommended implementation order

Do tasks in this order without skipping:
1. Task 1
2. Task 2
3. Task 3
4. Task 4
5. Task 5
6. Task 6
7. Task 7
8. Task 8
9. Task 9
10. Task 10
11. Task 11

## Handoff note

Plan complete. After this, implementation should proceed task-by-task with test-first discipline, keeping each commit narrowly scoped and each command runnable from the repo root.
