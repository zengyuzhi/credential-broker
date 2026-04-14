## Context

The 2026-04-14 Trail of Bits baseline left 13 CRIT-to-MEDIUM findings triaged rather than fixed. The zeroize group (7 of the 13) was deferred because a piecemeal fix would be half-work — `SecretStore::get` returning wiped bytes means nothing if `ResolvedCredential` immediately clones into an un-wiped `String`. This change addresses the zeroize findings together so the secret-path is hardened end-to-end in one commit, and bundles the four small input-validation fixes (SE-06..SE-09) that share the same audit provenance.

Current state after `6dd9f7e`:
- `subtle` crate in the workspace, `ConstantTimeEq` used in `auth.rs`.
- `SecretStore::put` removed from the trait.
- `docs/ROADMAP.md` has the 13 triaged items, each with `(audit: ... 2026-04-14 <id>)` tag.
- v0.1.0 shipped; `[Unreleased]` holds `--version` fix + four security bullets.

Workspace constraint: no `zeroize` anywhere yet. `sqlx` does transitively pull it, but we don't depend on it directly. First-party adoption is net-new.

## Goals / Non-Goals

**Goals:**
- Wipe the *primary* heap allocations holding raw secrets (API keys, lease tokens, session tokens) on drop.
- Reject invalid `ttl_minutes`, timestamp-overflow, latency-cast-overflow, and cost-accumulation drift at the type boundary rather than silently corrupting data.
- Close 13 of the 16 triaged audit items in one atomic change so the remaining backlog (3 items, all explicitly deferred with documented rationale) is reviewable at a glance.
- Ship as v0.1.1 candidate alongside the existing `[Unreleased]` fixes.

**Non-Goals:**
- Wiping buffers owned by `reqwest`, `tokio`, or `serde_json` — we don't control those allocations. Document as residual risk; don't pretend to fix.
- Defending against a root-level attacker with live ptrace/debugger access (that's a different threat model, out of reach for any userspace mitigation).
- Migrating off `askama` (SC-01). Separate change; axum-compat work is distinct from this hardening pass.
- Swapping `security-framework` for `keyring` (SC-03). Bundled with Linux port effort.
- Reducing bus-factor exposure on `reqwest` / `rpassword` (SC-02, SC-04). Monitor-only per original triage.
- Binary-level verification that the compiler didn't optimize zeroize writes away. Deferred to next audit pass with the scanner re-run.

## Decisions

### Decision 1 — Use `zeroize` crate, not a hand-rolled wiper

**Choice:** add `zeroize = { version = "1", features = ["derive"] }` to `[workspace.dependencies]`.

**Why not roll our own?** Writing a correct memory wipe is a known-hard problem — the compiler is free to elide "dead" stores to memory about to be freed. `zeroize` uses `core::ptr::write_volatile` + `compiler_fence(SeqCst)` to defeat LLVM's dead-store-elimination pass. Rolling our own would duplicate known-correct code with no upside.

**Alternatives considered:** `secrecy` crate (higher-level, has `ExposeSecret` trait). Rejected because it adds ceremony (`secret.expose_secret()` calls everywhere) for a property — wiping on drop — we can get from `zeroize` alone. We may adopt `secrecy` later if we need explicit exposure audit logging; for this change, the simpler primitive is correct.

### Decision 2 — Wrap, don't refactor to `Secret<T>` everywhere

**Choice:** Use `Zeroizing<String>` / `Zeroizing<Vec<u8>>` at the boundaries where secrets live; derive `#[derive(ZeroizeOnDrop)]` on structs (`ResolvedCredential`) that hold them. Do *not* introduce a workspace-wide `Secret<T>` newtype.

**Why:** `Zeroizing<T>` is a transparent wrapper that `Deref`s to `T` — call sites using `&secret` or `.as_str()` keep compiling. A `Secret<T>` newtype would force every call site through a getter, touching dozens of files for a property already guaranteed by `Zeroizing`.

**Alternatives considered:** force-through-`SecretString` (from `secrecy`). Rejected for the same reason as Decision 1 — churn without marginal benefit at this stage.

### Decision 3 — `SecretStore::get` trait signature changes

**Choice:** change the trait return from `anyhow::Result<String>` to `anyhow::Result<Zeroizing<String>>`.

**Why:** the trait is the API seam. If `get` returns a bare `String`, every caller must remember to wrap it — precisely the mistake the audit flagged. By hoisting the wrapper into the signature, we make the safe path the easy path.

**Breaking scope:** internal only (no external SemVer contract on these crates). The implementation change is `Ok(String::from_utf8(bytes).into())` — `Zeroizing` has a `From<T>` impl for its inner type.

### Decision 4 — `issue_lease` ttl validation via `NonZeroU32`

**Choice:** change the `ttl_minutes` parameter type from `i64` to `std::num::NonZeroU32`. Callers construct via `NonZeroU32::new(60).expect("nonzero")` or propagate an error at the CLI-arg parsing layer.

**Why:** push validation to the type system. The current `i64` accepts zero (immediate expiry), negative (panic on `Duration::from_secs`), and values beyond `u32::MAX` minutes (timestamp overflow). `NonZeroU32` eliminates all three classes at construction. Callers that receive user input handle the conversion explicitly; internal callers with constants use the infallible path.

**Alternatives considered:** `Result<Lease, LeaseError>` with runtime validation. Rejected because it pushes the failure case into the Happy Path rather than the boundary, and makes every existing caller thread a new error case.

### Decision 5 — Timestamp overflow: propagate `VaultError`, not `unwrap_or(now)`

**Choice:** replace `checked_add_signed(duration).unwrap_or(now)` with `checked_add_signed(duration).ok_or(VaultError::TimestampOverflow)?`.

**Why:** the current fallback silently produces an immediately-expired session — *technically* fail-closed, but callers have no signal to log or retry. Adding a variant to `VaultError` (already exists in `vault-core`) gives telemetry and callers a clean error.

**Alternatives considered:** panic. Rejected — a server-side panic on a pathological duration shouldn't crash the whole daemon for other sessions.

### Decision 6 — Latency cast saturates, doesn't truncate

**Choice:** replace `(duration.as_nanos() / 1_000) as i64` with `i64::try_from(duration.as_micros()).unwrap_or(i64::MAX)`.

**Why:** `i64::MAX` microseconds ≈ 292,471 years — any real request that hits this value has a telemetry bug that deserves a sticky max-value flag, not a silent wrap into a nonsense small number. Saturating is the honest representation.

### Decision 7 — Monetary values: `i64` microdollars

**Choice:** `estimated_cost_usd: Option<f64>` → `estimated_cost_micros: Option<i64>`. 1 microdollar = $0.000001. `i64` covers ±$9.2 quadrillion — no overflow risk for any realistic usage.

**Why:** f64 rounds cents across thousands of additions; integer accumulation is exact. Microdollar granularity preserves pricing-model precision (OpenAI bills at $0.00001/token tier).

**Migration:** SQLite migration `0008_usage_events_cost_micros.sql`:
```sql
ALTER TABLE usage_events ADD COLUMN estimated_cost_micros INTEGER;
UPDATE usage_events
  SET estimated_cost_micros = CAST(estimated_cost_usd * 1000000 AS INTEGER)
  WHERE estimated_cost_usd IS NOT NULL;
ALTER TABLE usage_events DROP COLUMN estimated_cost_usd;
```

SQLite 3.35+ supports `DROP COLUMN` (macOS ships 3.43+). Verified via `sqlite3 --version` ≥ 3.35 on our macOS 15 target.

### Decision 8 — Session token in JSON response: wipe primary, accept serde copy

**Choice:** `login_handler` in `auth.rs` holds the session token as `Zeroizing<String>` in its local scope; when constructing the JSON response, serde serializes into a new (un-wiped) buffer owned by axum. Accept this — document the residual gap, don't bend over backwards.

**Why:** wrapping the serde output would require a custom `Serialize` impl or a fork of the response-type — 50x the code for a 2x reduction in exposure. The *primary* allocation (where the token lives longest, ~10ms window) is wiped; the serde copy (microseconds before the TCP send consumes it) is short-lived.

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| `Zeroizing<String>` clone creates an un-wiped copy | Audit all `.clone()` calls on secret values during implementation; prefer passing `&str` references |
| Compiler optimizes away the wipe | `zeroize` uses `write_volatile` + `compiler_fence` — this is literally the crate's purpose. Follow-up TOB scanner pass verifies at binary level |
| `NonZeroU32` change breaks 12+ test sites | Tests use `NonZeroU32::new(60).unwrap()` — mechanical edit, ~5 min |
| `estimated_cost_usd` → `estimated_cost_micros` breaks dashboard template | Template consumes `f64` for display; add a `cost_display_usd()` method returning `f64` on the summary struct. Query-time only, no accumulation |
| Migration corrupts cost data | Backfill uses `CAST(... AS INTEGER)` which truncates — acceptable for cost display. Save `.local/vault.db` to `.local/vault.db.pre-008.bak` in migration script doc, user can rollback if needed |
| Axum response body for session login still holds un-wiped bytes | Documented as residual gap in SUMMARY.md and CHANGELOG. Full fix requires custom serializer, out of scope |
| `reqwest` keeps upstream API key in HTTP/2 frame buffer post-send | Documented; fix requires forking `reqwest` or switching to `hyper` directly — both L-sized, not justified by marginal threat model reduction |
| TOB re-audit during v0.1.1 release finds new CRIT/HIGH | Release gate per `release-process` spec — would block v0.1.1 until Fix/Triage/Accept cycle. Expected: 0 new findings given this is a hardening-only change |

## Migration Plan

1. Add `zeroize` to `[workspace.dependencies]`; add per-crate `zeroize.workspace = true` to Cargo.tomls that will use it.
2. Change `SecretStore::get` trait signature; update `MacOsKeychainStore::get` impl; fix call sites (5 known call sites across `vault-cli`, `vaultd`).
3. Derive `ZeroizeOnDrop` on `ResolvedCredential`; wrap `secret_value` in `Zeroizing`.
4. Wrap proxy `Bearer <token>` header value.
5. Change `issue_lease` signature to `NonZeroU32`; fix ~4 call sites.
6. Wrap lease `raw_token` and session `raw_token` in `Zeroizing`.
7. Replace `checked_add_signed(...).unwrap_or(now)` with `?` propagation.
8. Replace latency cast with saturating `try_from`.
9. Add SQLite migration `0008_usage_events_cost_micros.sql`; rename field in `UsageEvent`; update dashboard display helper.
10. `cargo test --workspace` → all green. `cargo clippy --workspace --all-targets -- -D warnings` → 0 warnings. `cargo fmt --all -- --check` → clean.
11. Update CHANGELOG `[Unreleased] → Security` with 13-finding summary.
12. Close ROADMAP entries (remove or mark as shipped).
13. Commit as single atomic change; push to main.

**Rollback:** git revert the single commit. SQLite migration is destructive (old column dropped) — user with pre-0.1.1 `.local/vault.db` who reverts binary loses cost data for display, but the raw bytes in `usage_events` rows are otherwise intact. Document as "pre-0.1.1 cost data shows N/A after revert".

## Open Questions

- **Should `subtle::ConstantTimeEq` get the same wrapping treatment?** The `subtle` crate already has `CtOption<T>` for constant-time Option handling, but our usage is exclusively on hash-compare return values that aren't secrets themselves. Leaving as-is.
- **Is `Zeroizing<String>` enough, or do we need `secrecy::SecretString` for explicit-expose semantics?** This change takes the simpler path. If we later want mandatory `.expose_secret()` audit points, we can layer `secrecy` on top without breaking `Zeroizing`.
- **Does the dashboard SSE stream leak secrets?** Audited — SSE payloads are enumerated event types (credential-updated, profile-updated, session-expired) with IDs and timestamps only. No raw secret material transits SSE. No change needed.
