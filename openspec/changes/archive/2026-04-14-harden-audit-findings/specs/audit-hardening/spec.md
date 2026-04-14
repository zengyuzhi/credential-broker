## ADDED Requirements

### Requirement: Secrets read from the keychain SHALL be wiped on drop

The `SecretStore::get` trait method in `crates/vault-secrets/src/lib.rs` SHALL return `anyhow::Result<Zeroizing<String>>` rather than `anyhow::Result<String>`. Any in-memory copy of a secret value derived from this return type MUST either remain wrapped in `Zeroizing` or live inside a struct that derives `ZeroizeOnDrop`.

#### Scenario: Keychain fetch wipes bytes on drop

- **WHEN** a caller invokes `store.get(service, account)` on `MacOsKeychainStore` for a valid entry
- **THEN** the returned value dereferences to `&str` containing the secret
- **AND** when the returned `Zeroizing<String>` goes out of scope, the underlying heap bytes are overwritten with zeros before the allocation is freed

#### Scenario: Trait signature prevents un-wiped returns

- **WHEN** a future `SecretStore` implementation is added (e.g., Linux `secret-service`)
- **THEN** the Rust compiler SHALL reject any `get` impl that returns a bare `String`
- **AND** the implementer is forced through `Zeroizing<String>` or an equivalent wiping wrapper

### Requirement: Resolved credential structs SHALL wipe secret fields on drop

The `ResolvedCredential` struct in `crates/vault-core/src/provider.rs` and any equivalent per-binding resolution struct SHALL wipe its secret-bearing fields (the values of `fields: HashMap<String, String>`, and any sibling field carrying raw secret material) when dropped. This SHALL be implemented either via `#[derive(zeroize::ZeroizeOnDrop)]` OR via a manual `Drop` impl that calls `Zeroize::zeroize` on each secret value. A manual impl is required when fields use container types (e.g., `HashMap<String, String>`) that `zeroize` 1.x does not provide a blanket `Zeroize` impl for.

#### Scenario: `vault run` wipes resolved secrets after spawn

- **WHEN** `vault run --profile foo -- cmd` completes the `Command::spawn` call
- **THEN** the `ResolvedCredential` instances go out of scope at function return
- **AND** every secret value in `ResolvedCredential.fields` is zeroed before the heap allocation is freed

#### Scenario: Drop on early return path

- **WHEN** `vault run` errors partway through env_map construction (e.g., missing Keychain entry)
- **THEN** the partial `Vec<ResolvedCredential>` built so far is dropped
- **AND** every already-resolved secret is wiped before propagating the error

### Requirement: Lease tokens and session tokens SHALL be returned as `Zeroizing<String>`

`vault_policy::lease::issue_lease` SHALL return `anyhow::Result<(Lease, Zeroizing<String>)>` where the second element is the raw (pre-hash) token. The dashboard `login_handler` in `crates/vaultd/src/auth.rs` SHALL hold its primary session-token allocation in a `Zeroizing<String>` binding.

#### Scenario: Lease issue wipes raw token

- **WHEN** a caller invokes `issue_lease(profile_id, NonZeroU32::new(60).unwrap())`
- **THEN** the returned tuple's second element is `Zeroizing<String>`
- **AND** once the caller's binding goes out of scope, the raw token bytes are zeroed

#### Scenario: Dashboard login wipes session token

- **WHEN** `POST /ui/login` succeeds
- **THEN** the `raw_token` binding inside `login_handler` is `Zeroizing<String>`
- **AND** that primary allocation is wiped when the handler returns (Serde's response-body copy remains un-wiped and is documented as a residual gap, not a defect)

### Requirement: HTTP proxy `Authorization` header values SHALL be wiped

`crates/vaultd/src/routes/proxy.rs` SHALL store the `Bearer <api-key>` header string in a `Zeroizing<String>` binding between construction and the `reqwest` send call. The binding MUST be dropped as soon as the outgoing request is dispatched.

#### Scenario: Proxy request wipes header string

- **WHEN** `POST /v1/proxy/openai/chat/completions` builds the upstream request
- **THEN** the Bearer-header value is held in `Zeroizing<String>`
- **AND** after the `reqwest::Client::send` call returns, the local allocation is zeroed on drop

### Requirement: Keychain write stdin buffer SHALL be wiped

`MacOsKeychainStore::put_with_access` in `crates/vault-secrets/src/keychain.rs` SHALL hold the secret bytes being piped to `/usr/bin/security` in a `Zeroizing<Vec<u8>>` or equivalent wiping wrapper between `stdin.write_all(...)` and the subprocess wait.

#### Scenario: `vault credential add` wipes stdin buffer

- **WHEN** the CLI writes a new secret to the keychain
- **THEN** the in-process buffer carrying the bytes before they reach `/usr/bin/security`'s stdin is wrapped in `Zeroizing<Vec<u8>>`
- **AND** once the subprocess has consumed the pipe, the buffer is zeroed on drop

### Requirement: Lease TTL SHALL be validated at the type boundary

`vault_policy::lease::issue_lease` SHALL accept `ttl_minutes: std::num::NonZeroU32` rather than `i64`. Zero, negative, and `u32::MAX`-exceeding values MUST be rejected at caller construction time, not at lease-issuance time.

#### Scenario: Zero TTL rejected at construction

- **WHEN** a caller attempts `NonZeroU32::new(0)`
- **THEN** the result is `None`
- **AND** `issue_lease` cannot be called with an immediately-expired TTL

#### Scenario: Valid TTL accepted

- **WHEN** a caller invokes `issue_lease(profile_id, NonZeroU32::new(60).unwrap())`
- **THEN** the function returns a Lease with `expires_at = now + 60 minutes`

#### Scenario: CLI argument parsing validates at the edge

- **WHEN** the CLI parses a `--ttl-minutes` argument from the user
- **THEN** the parser returns an error for 0 or negative input before `issue_lease` is called

### Requirement: Timestamp-overflow SHALL return an error, not silently substitute `now`

Every `checked_add_signed(duration)` call in `crates/vaultd/src/auth.rs` and `crates/vault-db/src/ui_sessions.rs` SHALL propagate a `VaultError::TimestampOverflow` (or equivalent typed error) when the addition overflows, rather than calling `unwrap_or(now)`.

#### Scenario: Overflow propagates as error

- **WHEN** `chrono::Utc::now().checked_add_signed(Duration::minutes(i64::MAX))` is evaluated in session-expiry computation
- **THEN** the handler returns an error response rather than creating an immediately-expired session

#### Scenario: Normal arithmetic returns the computed timestamp

- **WHEN** the addition does not overflow
- **THEN** the computed `DateTime<Utc>` is returned via `Ok(...)`

### Requirement: Latency microsecond cast SHALL saturate, not truncate

`crates/vaultd/src/routes/proxy.rs` SHALL convert `u128` microsecond values to `i64` via `i64::try_from(micros).unwrap_or(i64::MAX)`, never via `as i64`.

#### Scenario: Normal latency casts losslessly

- **WHEN** a request takes 1_500_000 microseconds
- **THEN** `latency_us` stored in the usage event equals `1_500_000i64`

#### Scenario: Overflow saturates to i64::MAX

- **WHEN** a hypothetical u128 latency value exceeds `i64::MAX`
- **THEN** the stored `latency_us` equals `i64::MAX` rather than a truncated small number

### Requirement: Monetary cost SHALL be stored as integer microdollars

`vault_core::models::UsageEvent` SHALL carry `estimated_cost_micros: Option<i64>` rather than `estimated_cost_usd: Option<f64>`. The SQLite schema SHALL have the column `estimated_cost_micros INTEGER` via migration `0008_usage_events_cost_micros.sql`. Display code that needs a dollar value for UI SHALL convert via `as f64 / 1_000_000.0` at render time only, never accumulating in `f64`.

#### Scenario: Migration backfills existing cost data

- **WHEN** a pre-0.1.1 `.local/vault.db` containing `estimated_cost_usd = 0.003` rows is opened by a 0.1.1 binary
- **THEN** migration `0008` runs and the row now has `estimated_cost_micros = 3000`
- **AND** the old `estimated_cost_usd` column is dropped

#### Scenario: New events write microdollars directly

- **WHEN** the proxy records a usage event with a cost estimate of 3000 microdollars
- **THEN** the `estimated_cost_micros` column contains the integer 3000
- **AND** no `f64` intermediate is persisted

#### Scenario: Dashboard displays dollars without drift

- **WHEN** the stats page renders the per-provider cost column
- **THEN** the displayed value is computed as `(sum_of_micros as f64) / 1_000_000.0`
- **AND** the accumulation (`SUM`) happens in SQL as `INTEGER`, not as `REAL`

### Requirement: Change SHALL retire triaged ROADMAP entries

When this change is archived, `docs/ROADMAP.md` SHALL no longer contain the 13 bullets tagged with `(audit: ... 2026-04-14 ZA-0001..0007 | SE-05..09)`. Each retired entry SHALL be referenced by commit SHA in the CHANGELOG `[Unreleased] → Security` section.

#### Scenario: ROADMAP shows no stale audit-triaged entries for this batch

- **WHEN** the change is archived to `openspec/changes/archive/YYYY-MM-DD-harden-audit-findings/`
- **THEN** `docs/ROADMAP.md` does not contain any `(audit: ... 2026-04-14 ZA-0001)` through `ZA-0007` bullets
- **AND** does not contain `(audit: ... 2026-04-14 SE-05)` through `SE-09` bullets
- **AND** the remaining audit bullets (SC-01, SC-02, SC-03, SC-04, SE-10, SE-11, SE-12) are preserved with their original triage tags

#### Scenario: CHANGELOG cross-references commit

- **WHEN** the fixes commit lands on main
- **THEN** `CHANGELOG.md` `[Unreleased] → Security` contains at least one bullet per retired finding
- **AND** each bullet references the commit SHA and the finding ID (e.g., "SE-06")
