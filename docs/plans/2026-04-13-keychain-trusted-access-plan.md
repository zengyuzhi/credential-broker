# Keychain Trusted Access Implementation Plan

> For Hermes: Use subagent-driven-development to implement this plan task-by-task.

Goal: Make vault-cli store macOS Keychain items with pre-authorized access for the vault-cli binary so `vault run` works without a first-run hang, while keeping the new non-interactive failure path as a clear fallback.

Architecture: Keep the current split of metadata in SQLite and secret material in macOS Keychain. Extend `vault-secrets` so writes can attach a trusted-application ACL for the resolved vault-cli executable, and keep reads non-interactive so missing authorization becomes an explicit error instead of a hang. Use the already-fixed workspace-absolute SQLite path as the single source of truth for all CLI commands.

Tech Stack: Rust 2024, clap, tokio, security-framework, macOS Keychain Services, sqlx + SQLite.

Current known state:
- `current_database_url()` now resolves to the workspace DB at `/Users/zengy/credential-broker/.local/vault.db` unless overridden.
- `vault run` no longer hangs silently; it fails at Keychain read with a clear auth error.
- `vault-cli` tests pass locally: `cargo test -p vault-cli -- --nocapture`.

Out of scope:
- HTTP proxy mode
- Team / multi-user secrets
- Key rotation UX
- Windows or Linux secret-store backends

---

## Acceptance criteria

By the end of this plan, all of the following should be true:
- `vault credential add openai work-main` stores the secret in Keychain with trusted access for the local `vault-cli` executable.
- `vault run --profile coding --agent demo -- <cmd>` succeeds without hanging when the stored item was created by the updated CLI.
- If the ACL cannot be applied or the item was created by an older build, `vault run` fails fast with an actionable message instead of waiting forever.
- Running from either `/Users/zengy/credential-broker` or `/Users/zengy/credential-broker/crates/vault-cli` uses the same SQLite file.
- Focused tests cover path resolution and database selection logic; full `vault-cli` tests remain green.

---

## Shared conventions for every task

- Always run `cargo fmt --all` after changing Rust files.
- Run the narrowest possible test first, then `cargo test -p vault-cli -- --nocapture`.
- Never reintroduce an interactive Keychain read in `vault run`; reads must stay non-interactive.
- Keep all user-facing error strings actionable: state what failed and what the user should do next.
- Prefer adding new helpers in `vault-secrets` rather than scattering macOS Security logic across CLI command files.

---

### Task 1: Introduce a trusted-application abstraction in vault-secrets

Objective: Create a small, testable layer that resolves which executable paths should be granted Keychain access.

Files:
- Modify: `crates/vault-secrets/Cargo.toml`
- Modify: `crates/vault-secrets/src/lib.rs`
- Create: `crates/vault-secrets/src/access.rs`
- Test: `crates/vault-secrets/src/access.rs`

Step 1: Write a failing test for executable-path resolution

Add tests that verify:
- a direct binary path like `/Users/zengy/credential-broker/target/debug/vault-cli` is preserved
- a symlinked or relative path is canonicalized
- duplicate paths are removed
- an env override like `VAULT_TRUSTED_APP_PATHS` can append extra paths for recovery/manual testing

Suggested test shape:
```rust
#[test]
fn trusted_application_paths_should_include_current_exe() {
    let paths = trusted_application_paths_for(
        "/tmp/work/target/debug/vault-cli",
        Some("/tmp/work/target/debug/vault-cli:/Applications/iTerm.app"),
    )
    .unwrap();

    assert!(paths.iter().any(|p| p.ends_with("target/debug/vault-cli")));
    assert_eq!(paths.len(), 2);
}
```

Step 2: Run the focused test and confirm failure

Run:
`cargo test -p vault-secrets trusted_application_paths_should_include_current_exe -- --nocapture`

Expected: fail because the helper module does not exist yet.

Step 3: Implement the helper module

Implementation notes:
- Add a new `TrustedApplicationSpec` helper or equivalent simple API in `crates/vault-secrets/src/access.rs`
- Start with a pure function:
```rust
pub fn trusted_application_paths_for(
    current_exe: impl AsRef<Path>,
    env_override: Option<&str>,
) -> anyhow::Result<Vec<PathBuf>>
```
- Canonicalize existing filesystem paths when possible.
- Ignore blank override entries.
- Deduplicate via a `BTreeSet<PathBuf>` or equivalent.
- Re-export the helper from `crates/vault-secrets/src/lib.rs` if other crates need it.
- If macOS ACL APIs require extra bindings, add the minimal dependency in `crates/vault-secrets/Cargo.toml` now.

Step 4: Re-run the focused test

Run:
`cargo test -p vault-secrets trusted_application_paths_should_include_current_exe -- --nocapture`

Expected: pass.

Step 5: Commit

Run:
`git add crates/vault-secrets && git commit -m "feat: add trusted application path resolver"`

---

### Task 2: Add a Keychain write path that attaches trusted applications

Objective: When a credential is created, write the Keychain item with explicit trusted access for the resolved vault-cli executable.

Files:
- Modify: `crates/vault-secrets/src/keychain.rs`
- Modify: `crates/vault-secrets/src/lib.rs`
- Test: `crates/vault-secrets/src/keychain.rs`
- Modify if needed: `crates/vault-secrets/Cargo.toml`

Step 1: Write a failing test for the write-options builder

Because live Keychain ACL writes are hard to unit-test portably, test the configuration helper first.

Add a test for a helper shaped like:
```rust
#[cfg(target_os = "macos")]
fn generic_password_options_with_trusted_apps(
    service: &str,
    account: &str,
    trusted_apps: &[PathBuf],
) -> anyhow::Result<PasswordOptions>
```

Expected behaviors:
- service and account are preserved
- trusted app list must not be empty
- duplicate app paths are removed before building ACL state

Step 2: Run the focused test and confirm failure

Run:
`cargo test -p vault-secrets generic_password_options_with_trusted_apps -- --nocapture`

Expected: fail because the helper does not exist.

Step 3: Implement the trusted-write path

Implementation notes:
- Extend `MacOsKeychainStore` with a new write API that accepts trusted app paths, for example:
```rust
async fn put_with_access(
    &self,
    service: &str,
    account: &str,
    secret: &str,
    trusted_apps: &[PathBuf],
) -> anyhow::Result<String>
```
- Keep the existing `put()` as a thin wrapper or migrate call sites to the new API.
- Use macOS Security APIs to create an ACL/access object for the trusted applications before storing the password item.
- If the high-level `security-framework` crate is insufficient, add only the required `security-framework-sys` calls instead of shelling out to the `security` command.
- Preserve the existing `service:account` `secret_ref` format.
- Keep read and delete behavior unchanged in this task.

Suggested call site shape inside `put()` or the new helper:
```rust
let trusted_apps = trusted_application_paths()?;
let options = generic_password_options_with_trusted_apps(service, account, &trusted_apps)?;
set_generic_password_options(secret.as_bytes(), options)
    .with_context(|| format!("failed to store secret for {service}/{account}"))?;
```

Step 4: Re-run the focused tests

Run:
`cargo test -p vault-secrets -- --nocapture`

Expected: unit tests pass. If a live Keychain integration test is added, gate it behind `#[ignore]` and run it manually.

Step 5: Commit

Run:
`git add crates/vault-secrets && git commit -m "feat: store keychain items with trusted vault-cli access"`

---

### Task 3: Thread trusted access through credential add

Objective: Ensure new credentials are written with the trusted-access path rather than the old plain `put()` path.

Files:
- Modify: `crates/vault-cli/src/commands/credential.rs`
- Test: `crates/vault-cli/src/commands/credential.rs`
- Reference: `crates/vault-secrets/src/keychain.rs`

Step 1: Write a failing CLI-level regression test

Add a focused test around the path-selection / write-call boundary. Since the real Keychain should not be touched in a unit test, extract a helper that can be tested without live Keychain I/O.

Example seam to test:
```rust
fn keychain_account_and_access_targets(
    credential_id: Uuid,
    field_name: &str,
    current_exe: &Path,
    env_override: Option<&str>,
) -> anyhow::Result<(String, Vec<PathBuf>)>
```

Expected assertions:
- the account string remains `credential:<uuid>:<field>`
- the trusted path list contains the resolved `vault-cli` path

Step 2: Run the focused test and confirm failure

Run:
`cargo test -p vault-cli keychain_account_and_access_targets -- --nocapture`

Expected: fail because the helper does not exist.

Step 3: Implement the new write path

Implementation notes:
- Keep `build_keychain_account()` unchanged.
- Replace the current macOS branch in `add_credential()` with the trusted-access write API from `vault-secrets`.
- Do not store trusted paths in SQLite; they belong only to Keychain ACL configuration.
- Keep user-visible success output unchanged unless a new warning is needed.

Step 4: Re-run the focused tests, then the full vault-cli tests

Run:
`cargo test -p vault-cli -- --nocapture`

Expected: pass.

Step 5: Commit

Run:
`git add crates/vault-cli && git commit -m "feat: write credentials with trusted keychain access"`

---

### Task 4: Keep non-interactive reads but upgrade failure messaging

Objective: Preserve the new no-hang behavior and turn auth failures into clear recovery instructions.

Files:
- Modify: `crates/vault-secrets/src/keychain.rs`
- Modify: `crates/vault-cli/src/commands/run.rs`
- Test: `crates/vault-cli/src/commands/run.rs`

Step 1: Write a failing test for error translation

Add a focused test for a helper like:
```rust
fn explain_keychain_read_error(message: &str) -> String
```

Expected output should mention at least one of:
- re-add the credential with the updated CLI
- authorize the `target/debug/vault-cli` binary manually if this was an older entry
- use `VAULT_TRUSTED_APP_PATHS` only for recovery/debugging

Step 2: Run the focused test and confirm failure

Run:
`cargo test -p vault-cli explain_keychain_read_error -- --nocapture`

Expected: fail because the helper does not exist.

Step 3: Implement the translation layer

Implementation notes:
- Keep `SecKeychain::disable_user_interaction()` in `MacOsKeychainStore::get()`.
- Detect auth-related Keychain failures and wrap them with a more actionable message.
- Keep the low-level cause attached for debugging.
- In `run.rs`, keep the debug logs behind `VAULT_DEBUG_RUN`; do not print secrets or secret lengths in normal mode.

Suggested message shape:
```text
Keychain access for this credential is not authorized for the current vault-cli binary.
Re-add the credential with the updated CLI, or manually allow target/debug/vault-cli in Keychain Access for this item.
```

Step 4: Re-run focused tests, then the full suite

Run:
`cargo test -p vault-cli -- --nocapture`

Expected: pass.

Step 5: Commit

Run:
`git add crates/vault-cli crates/vault-secrets && git commit -m "fix: surface actionable keychain authorization errors"`

---

### Task 5: Manual verification with a fresh credential

Objective: Prove the trusted-access flow works end-to-end on the real macOS machine.

Files:
- No required source changes
- Optional notes: `docs/plans/2026-04-13-keychain-trusted-access-plan.md`

Step 1: Create a fresh test credential with the updated build

Run interactively:
`cargo run -p vault-cli -- credential add openai trusted-e2e --kind api_key --env work`

Expected: success message with a new credential id.

Step 2: Bind it to a fresh profile

Run:
`cargo run -p vault-cli -- profile create trusted-coding`
`cargo run -p vault-cli -- profile bind trusted-coding openai <new-credential-id> --mode inject`

Expected: profile bind succeeds.

Step 3: Verify env injection end-to-end

Run:
`VAULT_DEBUG_RUN=1 cargo run -p vault-cli -- run --profile trusted-coding --agent demo --project verify -- python3 -c 'import os; print(bool(os.getenv("OPENAI_API_KEY"))); print(os.getenv("VAULT_PROFILE")); print(os.getenv("VAULT_AGENT")); print(os.getenv("VAULT_PROJECT")); print(bool(os.getenv("VAULT_LEASE_TOKEN")))'`

Expected:
- no hang
- `True` for `OPENAI_API_KEY`
- profile/agent/project values print correctly
- `True` for `VAULT_LEASE_TOKEN`

Step 4: Verify the old credential failure path still explains recovery

Run the same `vault run` flow against the legacy `work-main` credential if it still exists.

Expected: fast failure with actionable remediation text, not a timeout.

Step 5: Commit docs or follow-up notes if needed

Run:
`git add docs/plans && git commit -m "docs: record keychain trusted access verification notes"`

---

## Final verification checklist

Before marking the work complete:
- `cargo fmt --all`
- `cargo test -p vault-secrets -- --nocapture`
- `cargo test -p vault-cli -- --nocapture`
- `cargo run -p vault-cli -- profile show coding`
- `cargo run -p vault-cli -- credential list`
- one successful fresh-credential `vault run`
- one intentional old-credential failure showing the new recovery message

## Execution handoff

Plan saved. Recommended execution order is exactly the task order above: path helpers first, trusted write second, CLI wiring third, error UX fourth, manual verification last.