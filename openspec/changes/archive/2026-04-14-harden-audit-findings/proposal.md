## Why

The 2026-04-14 Trail of Bits baseline surfaced 13 actionable CRITICAL-to-MEDIUM findings that were triaged into `docs/ROADMAP.md` rather than fixed in the audit change (which was scope-capped at CRIT + HIGH-Fix-now). Seven of those findings collapse to the same root cause: secrets live in `String` / `Vec<u8>` that drop without wiping, leaving plaintext API keys, lease tokens, and session tokens in heap memory until the allocator reuses the pages. The remaining findings are small input-validation gaps (ttl, timestamps, latency, cost) that silently corrupt or panic rather than erroring. Shipping the CRIT + HIGH-Fix-now band without following through on the triaged remainder leaves the threat model half-addressed: an attacker with memory-read primitives (core dump, swap, debugger) can still recover raw secrets. This change closes the zeroize gap and the numeric-footgun gap in one pass so v0.1.1 carries a coherent hardening story, not a partial one.

## What Changes

- **Adopt the `zeroize` crate workspace-wide.** Add `zeroize = "1"` to `[workspace.dependencies]` with the `derive` feature.
- **`SecretStore::get` returns wrapped secrets.** Change return type from `String` to `Zeroizing<String>` so the keychain read is wiped on drop. Covers **ZA-0001**.
- **`ResolvedCredential` derives `ZeroizeOnDrop`.** The per-binding struct in `vault-cli/src/commands/run.rs` carrying `secret_value: String` fields is wiped when the command scope ends. Covers **ZA-0002, ZA-0006**.
- **Proxy `Authorization` header value is wiped after send.** `crates/vaultd/src/routes/proxy.rs` wraps the `format!("Bearer {secret}")` result in `Zeroizing<String>`. Covers **ZA-0003**.
- **Lease `raw_token` wrapped in `Zeroizing`.** `issue_lease()` in `vault-policy` returns `Zeroizing<String>`; callers receive wrapped value. Covers **ZA-0004**.
- **Dashboard session `raw_token` wrapped.** `login_handler` in `vaultd/src/auth.rs` wraps the session token. Serde-serialized response body still copies unwiped, but the *primary* allocation is now wiped — documented in design as best-effort. Covers **ZA-0005**.
- **`put_with_access` stdin buffer explicitly wiped.** `crates/vault-secrets/src/keychain.rs` holds the secret in a `Zeroizing<Vec<u8>>` during the pipe to `/usr/bin/security`. Covers **ZA-0007**.
- **BREAKING (internal): `issue_lease()` validates `ttl_minutes`.** Switch parameter from `i64` to `std::num::NonZeroU32` (or return `Result` on invalid input). Zero / negative / huge values now error at call site. Covers **SE-07**.
- **Timestamp overflow propagates, does not fall back to `now`.** `auth.rs` and `vault-db/src/ui_sessions.rs` replace `checked_add_signed(...).unwrap_or(now)` with explicit `Result` handling. Covers **SE-06**.
- **Latency cast saturates.** Replace `(duration.as_nanos() / 1_000) as i64` with `i64::try_from(micros).unwrap_or(i64::MAX)` in `vaultd/src/routes/proxy.rs`. Covers **SE-08**.
- **Monetary values move to integer microdollars.** `UsageEvent.estimated_cost_usd: Option<f64>` → `estimated_cost_micros: Option<i64>`. Migration adds column, backfills via `(f64 * 1_000_000.0) as i64` for existing rows, drops old column. Covers **SE-09**.

## Capabilities

### New Capabilities

- `audit-hardening`: Secrets in memory are wiped when no longer referenced; numeric inputs at lease/telemetry boundaries are validated or saturating rather than silently truncating or panicking.

### Modified Capabilities

_None._ The existing `security-audit-baseline` capability is unchanged — this change *satisfies* its triaged-items requirement rather than modifying its spec.

## Impact

- **Crates affected** (workspace-wide): `vault-core` (UsageEvent field rename), `vault-db` (schema migration for microdollars, timestamp Result handling), `vault-secrets` (trait return type change), `vault-policy` (lease signature), `vault-telemetry` (field rename), `vault-cli` (run.rs ResolvedCredential), `vaultd` (auth.rs, proxy.rs, ui_sessions.rs).
- **New workspace dep**: `zeroize = { version = "1", features = ["derive"] }`. Adds ~0 to compile time (already transitively pulled by `sha2` / other crates).
- **SQLite migration**: `0008_usage_events_cost_micros.sql` adds `estimated_cost_micros INTEGER` column, backfills from `estimated_cost_usd`, drops old column. Existing `.local/vault.db` files migrate automatically on first pool open.
- **Public API breakage**: `SecretStore::get` signature change (trait). `vault_policy::lease::issue_lease` signature change. `vault_core::UsageEvent` field rename. All internal — no external API stability guarantees exist yet (pre-1.0).
- **Test impact**: 73 existing tests; expect ~10 to need signature updates for new lease/secret return types.
- **ROADMAP entries closed**: 7 items retire from Near-term, 1 from Medium-term (SE-09).

## Out of Scope

- **SC-01 askama fork migration**: Tracked as its own change (`migrate-askama-fork`). Requires moving off `askama_axum 0.4` → archived — separate axum-compat investigation, not a simple dep swap.
- **SC-02 (`reqwest` bus-factor), SC-04 (`rpassword` bus-factor)**: Original triage disposition was *monitor-only*; this change does not alter that. Revisit at next audit baseline.
- **SC-03 (`security-framework` → `keyring`)**: Bundled with the Linux port effort per ROADMAP Medium-term; evaluating an alternative in isolation would produce churn without the multi-platform payoff.
- **External-buffer zeroize**: `reqwest` HTTP/2 frame buffers and `tokio::io::AsyncWrite` internal pipe buffers are outside our ownership. Documented as a known gap in SUMMARY.md's "Incomplete scans" — not addressable without upstream changes.
- **Binary-level verification** of the zeroize adoption (ZA phases 2–3 IR/ASM checks): deferred to the next audit pass after this change lands, where the scanner has something optimized-to-verify.

## Security Implications

**Threat model addressed:** local attacker with memory-read capability (post-mortem core dump, swap file, debugger attach, side-channel heap scraping) between the moment a secret is read from Keychain and the moment the process exits.

**Before this change:** API keys, lease tokens, and session tokens persist in heap memory as `String` allocations until the allocator reuses the pages — unbounded window, potentially hours for a long-running `vaultd`.

**After this change:** the *primary* allocations holding secrets are explicitly zeroed on drop. Derived copies that transit `reqwest`'s internal buffers or `serde`'s serialization buffer remain unaddressable (documented as the residual gap). The window shrinks from "process lifetime" to "single request duration".

**Non-goals:** this change does **not** defend against a root-level attacker who can ptrace the live process — that is out of scope for any userspace mitigation.

**New dep audit:** `zeroize` is a Trail of Bits-recommended crate, MIT/Apache-2.0, actively maintained, used by `ring`, `rustls`, `aws-sdk-rust`. Adds no runtime overhead when `ZeroizeOnDrop` is not exercised.
