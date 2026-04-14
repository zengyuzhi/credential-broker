## 1. Workspace wiring

- [x] 1.1 Add `zeroize = { version = "1", features = ["derive"] }` to `[workspace.dependencies]` in root `Cargo.toml`
- [x] 1.2 Add `zeroize.workspace = true` to `crates/vault-secrets/Cargo.toml`
- [x] 1.3 Add `zeroize.workspace = true` to `crates/vault-policy/Cargo.toml`
- [x] 1.4 Add `zeroize.workspace = true` to `crates/vault-cli/Cargo.toml`
- [x] 1.5 Add `zeroize.workspace = true` to `crates/vaultd/Cargo.toml`
- [x] 1.6 `cargo build --workspace` → compiles (no uses yet, just the dep exists)

## 2. SecretStore trait signature (ZA-0001)

- [x] 2.1 In `crates/vault-secrets/src/lib.rs`, change `SecretStore::get` return type to `anyhow::Result<zeroize::Zeroizing<String>>`
- [x] 2.2 Update `MacOsKeychainStore::get` impl in `crates/vault-secrets/src/keychain.rs` to return `Ok(String::from_utf8(bytes)?.into())` (the `.into()` boxes into `Zeroizing`)
- [x] 2.3 Update all call sites that bind the return value — grep for `store.get(` and `secret_store.get(` across `vault-cli` and `vaultd`
- [x] 2.4 `cargo build --workspace` → compiles

## 3. Keychain stdin buffer (ZA-0007)

- [x] 3.1 In `crates/vault-secrets/src/keychain.rs` `put_with_access`, wrap the secret bytes passed to `stdin.write_all` in `Zeroizing<Vec<u8>>`
- [x] 3.2 Add a unit test asserting the buffer is wiped (construct a `Zeroizing`, drop it, verify contents via unsafe ptr read is zeroed — or accept the crate's documented guarantee and skip)

## 4. ResolvedCredential wiping (ZA-0002, ZA-0006)

- [x] 4.1 In `crates/vault-cli/src/commands/run.rs`, add `#[derive(zeroize::ZeroizeOnDrop)]` to the `ResolvedCredential` struct
- [x] 4.2 Change `secret_value: String` to `secret_value: Zeroizing<String>` (or derive handles the `String` field directly — confirm with derive-macro doc)
- [x] 4.3 Update env_map build site to dereference `&*secret_value` when passing to `Command::envs`
- [x] 4.4 `cargo build -p vault-cli` → compiles

## 5. Proxy Authorization header (ZA-0003)

- [x] 5.1 In `crates/vaultd/src/routes/proxy.rs`, change `let auth_header = format!("Bearer {secret}");` to `let auth_header: Zeroizing<String> = format!("Bearer {secret}").into();`
- [x] 5.2 At the reqwest builder site, pass `auth_header.as_str()` rather than the owned `String`
- [x] 5.3 `cargo build -p vaultd` → compiles

## 6. Lease signature + raw token wiping (ZA-0004, SE-07)

- [x] 6.1 In `crates/vault-policy/src/lease.rs`, change `issue_lease` signature to `pub fn issue_lease(profile_id: &str, ttl_minutes: std::num::NonZeroU32) -> anyhow::Result<(Lease, Zeroizing<String>)>`
- [x] 6.2 Replace `Duration::minutes(ttl_minutes as i64)` internal use with `Duration::minutes(i64::from(ttl_minutes.get()))`
- [x] 6.3 Wrap the generated UUID token in `Zeroizing<String>` before returning
- [x] 6.4 Update `vault-cli/src/commands/run.rs` lease call: `issue_lease(profile.id, NonZeroU32::new(60).expect("60 is nonzero"))`
- [x] 6.5 Update any other `issue_lease` call sites (grep for `issue_lease(`)
- [x] 6.6 Update `vault-policy` unit tests to use `NonZeroU32`
- [x] 6.7 `cargo build --workspace` → compiles; `cargo test -p vault-policy` → all pass

## 7. Dashboard session token wiping (ZA-0005)

- [x] 7.1 In `crates/vaultd/src/auth.rs` `login_handler`, wrap the generated session raw token: `let raw_token: Zeroizing<String> = uuid::Uuid::new_v4().to_string().into();`
- [x] 7.2 Hash and persist via `&*raw_token`, return `raw_token` cloned into the Json response (Serde copy is the acknowledged residual)
- [x] 7.3 `cargo build -p vaultd` → compiles

## 8. Timestamp overflow → Result (SE-06)

- [x] 8.1 Add `TimestampOverflow` variant to `VaultError` enum in `crates/vault-core/src/error.rs`
- [x] 8.2 In `crates/vaultd/src/auth.rs`, replace every `.checked_add_signed(...).unwrap_or(now)` with `.checked_add_signed(...).ok_or(VaultError::TimestampOverflow)?`
- [x] 8.3 In `crates/vault-db/src/ui_sessions.rs`, same replacement on the ~line-86 site
- [x] 8.4 Map `VaultError::TimestampOverflow` to a 500 response in the auth handler error path
- [x] 8.5 `cargo test -p vaultd` → pass

## 9. Latency cast saturates (SE-08)

- [x] 9.1 In `crates/vaultd/src/routes/proxy.rs` line ~161, replace `(duration.as_nanos() / 1_000) as i64` with `i64::try_from(duration.as_micros()).unwrap_or(i64::MAX)`
- [x] 9.2 Add a unit test on the conversion helper (extract into a pure fn if inline)
- [x] 9.3 `cargo test -p vaultd` → pass

## 10. Cost micros migration (SE-09)

- [x] 10.1 Create `migrations/0008_usage_events_cost_micros.sql`:
  ```sql
  ALTER TABLE usage_events ADD COLUMN estimated_cost_micros INTEGER;
  UPDATE usage_events
    SET estimated_cost_micros = CAST(estimated_cost_usd * 1000000 AS INTEGER)
    WHERE estimated_cost_usd IS NOT NULL;
  ALTER TABLE usage_events DROP COLUMN estimated_cost_usd;
  ```
- [x] 10.2 In `crates/vault-core/src/models.rs`, rename `estimated_cost_usd: Option<f64>` → `estimated_cost_micros: Option<i64>`
- [x] 10.3 Update `crates/vault-db/src/usage_events.rs` `map_usage_event_row` to read the new column
- [x] 10.4 Update `crates/vault-telemetry/src/lib.rs` to write microdollars (convert from provider-adapter f64 at write time: `(cost * 1_000_000.0) as i64`)
- [x] 10.5 Update provider adapters in `crates/vault-providers/src/{openai,anthropic,twitterapi}.rs` — keep `f64` in the adapter output, telemetry converts at boundary
- [x] 10.6 In `crates/vault-db/src/stats.rs` (or wherever the rollup query lives), change `SUM(estimated_cost_usd)` to `SUM(estimated_cost_micros)`, update `StatsSummary` field
- [x] 10.7 Add `fn cost_display_usd(&self) -> Option<f64>` helper on `StatsSummary` returning `self.cost_micros.map(|m| m as f64 / 1_000_000.0)`
- [x] 10.8 Update `crates/vaultd/templates/stats.html` to call the new helper
- [x] 10.9 Update `vault stats` CLI command text output formatting
- [x] 10.10 `cargo test --workspace` → all pass; schema migration runs on fresh DB

## 11. Fixture DB migration test (SE-09)

- [x] 11.1 Create a test fixture `.local/vault-pre-008.db` with a sample row carrying `estimated_cost_usd = 0.003`
- [x] 11.2 Test opens the fixture, asserts migration runs, asserts `estimated_cost_micros = 3000`, asserts old column is gone
- [x] 11.3 Clean up fixture in test teardown

## 12. Verify

- [x] 12.1 `cargo test --workspace` → all tests pass
- [x] 12.2 `cargo clippy --workspace --all-targets -- -D warnings` → exit 0
- [x] 12.3 `cargo fmt --all -- --check` → exit 0
- [x] 12.4 Verified `vault --version` exits 0 and `vault stats --json` runs migration on existing `.local/vault.db` without error; `sqlite3 .local/vault.db '.schema usage_events'` confirms `estimated_cost_micros INTEGER` present and `estimated_cost_usd` dropped
- [ ] 12.5 `vault run` interactive smoke test (deferred — requires keychain-bound profile in this shell)
- [ ] 12.6 `vault serve` + dashboard PIN login (deferred — requires interactive browser session)

## 13. CHANGELOG + ROADMAP

- [x] 13.1 Update `CHANGELOG.md` `[Unreleased] → Security` with one bullet per retired finding (ZA-0001..0007, SE-05, SE-06, SE-07, SE-08, SE-09), each referencing the commit SHA (backfill after commit)
- [x] 13.2 Remove the 13 retired bullets from `docs/ROADMAP.md`: 7 zeroize-related (Near-term), SE-06/SE-07/SE-08 (Near-term S), SE-09 (Medium-term M), and the zeroize umbrella entry (`Adopt zeroize across secret paths L`)
- [x] 13.3 Verify remaining audit bullets (SE-10/11/12, SC-01/02/03/04, Node.js v5 bump, `cargo-audit`, etc.) are untouched

## 14. Commit

- [x] 14.1 Atomic commit `3af4386` landed: `feat(sec): adopt zeroize, validate lease TTL, saturate latency, microdollar cost` (29 files, +627 / −42)
- [x] 14.2 CHANGELOG bullets reference finding IDs; commit SHA cross-reference via git log rather than inline per-bullet SHA (one-commit-many-bullets case)
- [x] 14.3 Single commit covers all changes; no follow-up amend needed
- [ ] 14.4 `git push origin main` — awaiting user confirmation before pushing

## 15. Close the loop

- [ ] 15.1 All tasks above checked
- [ ] 15.2 Archive via `/opsx:archive harden-audit-findings` — spec sync promotes `audit-hardening` into `openspec/specs/`
