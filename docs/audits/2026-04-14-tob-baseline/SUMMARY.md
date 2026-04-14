# SUMMARY — Trail of Bits audit baseline, 2026-04-14

First audit pass on credential-broker, run against the v0.1.0 codebase
(commit `007998e` at pass start, pre-fixes).

Scanners invoked:
- `zeroize-audit@trailofbits/skills` — secret zeroization + compiler-removed wipes
- `supply-chain-risk-auditor@trailofbits/skills` — dependency health scoring
- `sharp-edges@trailofbits/skills` — error-prone API / footgun detector

Raw output per scanner in sibling files. This SUMMARY normalizes findings
across the three into a single severity scale and assigns a terminal
disposition per item.

## Severity rubric

| Level       | Meaning |
|-------------|---------|
| **CRITICAL** | RCE, credential theft, auth bypass, key leak to disk/log/network. Ship-stopper. |
| **HIGH**     | Privilege escalation, non-CT secret compare, missing zeroize on raw API keys, actively-exploited CVE in direct dep. |
| **MEDIUM**   | Defense-in-depth gap, easy-to-hit sharp edge with bounded impact, moderate-risk dep. |
| **LOW**      | Style/hygiene, misleading doc, minor footgun. |
| **INFO**     | Observation only. |

## Disposition taxonomy

- **Fix now** — code change landed in this change (commit SHA cited)
- **Triage** — copied to `docs/ROADMAP.md` with `(audit: <skill> 2026-04-14)` tag
- **Accept** — documented rationale; no code change

## Finding counts

| Severity | Fix now | Triage | Accept | Total |
|----------|---------|--------|--------|-------|
| CRITICAL | 1       | 0      | 0      | 1     |
| HIGH     | 3       | 6      | 0      | 9     |
| MEDIUM   | 0       | 7      | 1      | 8     |
| LOW      | 0       | 3      | 2      | 5     |
| INFO     | 0       | 0      | 3      | 3     |
| **Total** | **4**  | **16** | **6**  | **26** |

## CRITICAL

### SE-01 — Non-constant-time PIN hash comparison
- **Location:** `crates/vaultd/src/auth.rs:229`
- **Finding:** `hash(&body.pin) != session.pin_hash` uses default `String` inequality. An attacker who can submit many PIN guesses and measure HTTP response time builds a statistical timing oracle. The 5-attempt burn counter does not protect against timing leakage within the 5 attempts that are permitted.
- **Disposition:** **Fix now** — commit `6dd9f7e`
- **Fix:** switched to `subtle::ConstantTimeEq` on the raw hex bytes. Both operands are fixed-length blake3 hex (64 chars), so length-oracle is not a concern. Added workspace dep `subtle = "2"`.

## HIGH

### SE-02 — Non-constant-time CSRF token comparison
- **Location:** `crates/vaultd/src/auth.rs:355`
- **Finding:** Same timing-side-channel issue as SE-01 applied to the session CSRF token. The empty-string guard ran *after* the non-CT comparison, so an empty expected value leaked whether the header matched through timing before the empty-check could short-circuit.
- **Disposition:** **Fix now** — commit `6dd9f7e`
- **Fix:** reordered to `Option::None` / `is_empty()` short-circuits *before* the comparison; `ConstantTimeEq` used for the compare itself; distinct error branches for missing/empty vs mismatch.

### SE-03 — Rate-limit key derived from spoofable HTTP header
- **Location:** `crates/vaultd/src/auth.rs:132-137`
- **Finding:** The PIN challenge rate limiter keyed on `x-forwarded-for` with `host` fallback — both client-controlled. A single local process rotating the header gets unlimited PIN challenges, defeating the 3/min limit.
- **Disposition:** **Fix now** — commit `6dd9f7e`
- **Fix:** replaced header lookup with a fixed `"loopback"` key. vaultd binds to `127.0.0.1` only, so a single shared bucket is the correct granularity. Comment documents why.

### SE-04 — `SecretStore::put` stored secrets without Keychain ACL
- **Location:** `crates/vault-secrets/src/lib.rs` (trait), `crates/vault-secrets/src/keychain.rs:137-141` (impl)
- **Finding:** `SecretStore` trait exposed `put(service, account, secret)` with no trusted-app ACL — any app on the system could read the stored secret. The secure write path is `put_with_access` on the concrete struct. No production code called `put`, but its presence on the trait meant any future caller reaching for a write path would silently pick the insecure one.
- **Disposition:** **Fix now** — commit `6dd9f7e`
- **Fix:** removed `put` from the `SecretStore` trait and from the `MacOsKeychainStore` impl. The only supported write path is now `put_with_access` on the concrete struct (called from `vault-cli/commands/credential.rs`). Doc-comment on the trait explains the decision.

### ZA-0001 — Keychain `Vec<u8>` buffer not wiped after conversion
- **Location:** `crates/vault-secrets/src/keychain.rs:146-148` (`get`)
- **Finding:** `get_generic_password` returns `Vec<u8>`; we `String::from_utf8(bytes)` and return. The original `Vec<u8>` is consumed by `from_utf8` which moves ownership; the secret bytes now live in the `String`'s heap allocation and are never zeroed on drop.
- **Disposition:** **Triage** → ROADMAP Near-term (seeds `add-zeroize-to-secret-paths`)
- **Note:** deferred as part of the workspace-wide zeroize adoption work (see SE-05). Fixing in isolation would be half a solution.

### ZA-0002 — API key `String` persists in `env_map` after child spawn
- **Location:** `crates/vault-cli/src/commands/run.rs` (env_map build + Command::envs)
- **Finding:** `HashMap<String, String>` holds API keys in the parent process heap until function return. `Command::envs()` copies into the child env block but does not take ownership of the source map. After `spawn()`, the parent-side `HashMap` and its inner `String`s drop without zeroing.
- **Disposition:** **Triage** → ROADMAP Near-term (seeds `add-zeroize-to-secret-paths`)

### ZA-0003 — API key `String` injected into HTTP header without wipe
- **Location:** `crates/vaultd/src/routes/proxy.rs` (Authorization header build)
- **Finding:** `format!("Bearer {secret}")` creates a fresh `String` allocation carrying the raw API key. `reqwest` takes `&str`/`&[u8]` references into its internal buffer without taking ownership; after the request, the `format!` result drops without zeroing. Additionally, reqwest's internal HTTP/2 frame buffer carries the same bytes across ownership we cannot control.
- **Disposition:** **Triage** → ROADMAP Near-term (seeds `add-zeroize-to-secret-paths`)

### ZA-0004 — Lease `raw_token` not wiped after hash
- **Location:** `crates/vault-policy/src/lease.rs` (`issue_lease`)
- **Finding:** A fresh UUID token is generated, blake3-hashed for persistence, then returned to the caller as a raw `String`. After consumers are done with the raw token, it drops without zeroing. The hash is not a secret; the *raw* token is.
- **Disposition:** **Triage** → ROADMAP Near-term

### ZA-0005 — Dashboard session `raw_token` not wiped in `login_handler`
- **Location:** `crates/vaultd/src/auth.rs:248` (session token generation)
- **Finding:** Analogous to ZA-0004 — the session token generated on successful PIN login is returned in a `Json` response body and then dropped by Axum's serializer without zeroing. Serde does not clear buffers.
- **Disposition:** **Triage** → ROADMAP Near-term

### SC-01 — `askama` / `askama_axum` — pinned to archived upstream
- **Location:** workspace `Cargo.toml` via `crates/vaultd/Cargo.toml`
- **Finding:** We pin `askama = "0.12"` and `askama_axum = "0.4"` from the `djc/askama` fork, which is **archived**. No future security patches. The active successor is `askama-rs/askama` (also `rinja`, a rename/hard-fork).
- **Disposition:** **Triage** → ROADMAP Medium-term (seeds `migrate-askama-fork` — migration is non-trivial because `askama_axum 0.4` incompatibility with `axum 0.8` is the reason we're on this exact version set; see the CLAUDE.md gotcha)

## MEDIUM

### SE-06 — Timestamp overflow falls back to `now`
- **Location:** `crates/vaultd/src/auth.rs:155-157`, `crates/vault-db/src/ui_sessions.rs:~86`
- **Finding:** `checked_add_signed(...).unwrap_or(now)` silently replaces an overflowed expiry with the current time, which means the session is immediately expired — an implicit "fail closed" but with no signal to the caller.
- **Disposition:** **Triage** → ROADMAP Near-term `S`

### SE-07 — `ttl_minutes: i64` unchecked in `issue_lease`
- **Location:** `crates/vault-policy/src/lease.rs:19`
- **Finding:** Zero/negative/huge values are accepted without validation. Zero → immediately-expired lease. Negative → panics on `Duration::from_secs(negative)`. Huge → timestamp overflow (see SE-06).
- **Disposition:** **Triage** → ROADMAP Near-term `S` — either validate at call sites or switch to `NonZeroU32` at the type level.

### SE-08 — `u128 as i64` truncating cast in latency telemetry
- **Location:** `crates/vaultd/src/routes/proxy.rs:161`
- **Finding:** `latency_us = (duration.as_nanos() / 1_000) as i64`. For latencies > ~292 years (or any observed request that underflows/wraps), the cast truncates silently. Not exploitable but produces nonsense telemetry.
- **Disposition:** **Triage** → ROADMAP Near-term `S`

### SE-09 — `f64` for monetary accumulation (`estimated_cost_usd`)
- **Location:** `crates/vault-core/src/models.rs` (`UsageEvent.estimated_cost_usd: Option<f64>`)
- **Finding:** Floating-point accumulation of money loses cents after enough additions. Not exploitable; produces slowly-drifting cost rollups.
- **Disposition:** **Triage** → ROADMAP Medium-term `M` — switch to integer microdollars.

### ZA-0006 — `secret_value` copied through `ResolvedCredential.fields`
- **Location:** `crates/vault-cli/src/commands/run.rs` (ResolvedCredential build)
- **Finding:** Intermediate buffers carrying the secret are created during field resolution; each is dropped without zeroing.
- **Disposition:** **Triage** → ROADMAP Near-term (part of `add-zeroize-to-secret-paths`)

### ZA-0007 — `put_with_access` stdin buffer not explicitly wiped
- **Location:** `crates/vault-secrets/src/keychain.rs` (put_with_access)
- **Finding:** The secret is piped over stdin to `/usr/bin/security`; the tokio I/O buffer carrying the bytes is dropped without explicit zeroize.
- **Disposition:** **Triage** → ROADMAP Near-term (part of `add-zeroize-to-secret-paths`)

### SC-02 — `reqwest` solo-maintained; high-value TLS surface
- **Location:** workspace deps via `crates/vaultd/Cargo.toml` + `crates/vault-providers/Cargo.toml`
- **Finding:** reqwest has one primary maintainer (seanmonstar) and no SECURITY.md. The crate is otherwise mature and widely-used, but the bus factor is low for such a sensitive crate.
- **Disposition:** **Triage** → ROADMAP Speculative (monitor-only) — no alternative offers a better security trade-off today.

### SC-03 — `security-framework` solo-maintained; ~294 GitHub stars
- **Location:** `crates/vault-secrets/Cargo.toml`
- **Finding:** macOS-Keychain FFI bindings with low visibility. `keyring` is a higher-level alternative already considered in ROADMAP for the Linux port.
- **Disposition:** **Triage** → ROADMAP Medium-term — already tracked under "Linux port" umbrella; add an explicit `evaluate-keyring-alternative` bullet.

### SC-04 — `rpassword` solo-maintained; ~274 GitHub stars
- **Location:** `crates/vault-cli/Cargo.toml`
- **Finding:** Password-prompt crate with low visibility. Used in `vault credential add` to read secrets from stdin without echo. Alternative: std-library + termion, or vendor.
- **Disposition:** **Triage** → ROADMAP Speculative (monitor-only) — small surface area; low urgency.

## LOW

### SE-10 — `secret_ref` service component silently discarded
- **Location:** `crates/vaultd/src/routes/proxy.rs:209`, `crates/vault-cli/src/commands/run.rs:227`
- **Disposition:** **Triage** → ROADMAP Speculative `S`

### SE-11 — `VAULT_TRUSTED_APP_PATHS` paths passed unsanitized to `security` CLI
- **Location:** `crates/vault-secrets/src/access.rs`
- **Finding:** Env-var contents are passed as CLI args to `/usr/bin/security` without path validation. Exploitable only by a process that can already set the victim's env — local elevation gadget, not remote. Documented as "privileged config" but not enforced.
- **Disposition:** **Triage** → ROADMAP Near-term `S` — add absolute-path + file-existence check.

### SE-12 — CSRF/Origin extraction defaults to empty string
- **Location:** `crates/vaultd/src/auth.rs:339,352,354`
- **Finding:** Several header extractions `unwrap_or("")` at the edge, conflating missing and empty. The CSRF path was fixed as part of SE-02; the other two (Origin, auth header) remain.
- **Disposition:** **Triage** → ROADMAP Near-term `S`

### SC-05 — `sqlx` (patched CVE in current version)
- **Location:** `crates/vault-db/Cargo.toml`
- **Finding:** RUSTSEC-2024-0363 fixed in sqlx ≥ 0.8.1; we use `0.8.6` which includes the fix.
- **Disposition:** **Accept** — rationale: already on patched version; documented here so future maintainers see the scanner hit and recognize it's resolved.

### SC-06 — `dirs` crate (archived; transitive-only)
- **Location:** transitive via `vault-cli` (`dirs`), `tokio-stream`, etc.
- **Finding:** `dirs` is archived upstream. `directories` is the active successor.
- **Disposition:** **Accept** — rationale: transitive-only usage, no direct dependency. Our upstream crates control when to migrate; tracking at our level adds no value.

## INFO

### ZA-0008 — `debug_log` via `VAULT_DEBUG_RUN` may expose `account` names
- **Finding:** When `VAULT_DEBUG_RUN=1`, we log credential `account` names (not secrets). A hostile log reader could correlate account names with services, but no secret material leaks.
- **Disposition:** **Accept** — rationale: account names are not secrets. Users who set `VAULT_DEBUG_RUN` accept verbose logging.

### ZA-0009 — No `zeroize` crate dependency in any workspace member
- **Finding:** Observation underlying SE-05 / ZA-0001..0005.
- **Disposition:** **Accept** — rationale: duplicate finding, fully captured by the Triaged items above.

### SC-07..SC-16 — clean direct deps (tokio, axum, blake3, serde, clap, thiserror, anyhow, tracing, uuid, async-trait)
- **Finding:** No risk signals; multi-maintainer, active release cadence, no CVE history beyond patched.
- **Disposition:** **Accept** — rationale: no action implied.

## Incomplete scans

- **Zeroize-audit phases 2–3 (IR/ASM compiler analysis).** The scanner declined to run `cargo build` for binary inspection, so `OPTIMIZED_AWAY_ZEROIZE`, `STACK_RETENTION`, and `REGISTER_SPILL` checks are rated `likely` rather than `confirmed`. If we ever adopt `zeroize` workspace-wide, a follow-on pass should perform the binary-level verification.
- **Zeroize: external buffers beyond crate boundaries.** `reqwest`'s internal HTTP/2 frame buffers and `tokio::io::AsyncWrite`'s stdin pipe buffer are outside the scanner's reach. Secrets that transit these are assumed to drop unwiped.
- **Sharp-edges: vault-db sub-modules, vault-providers adapters, vaultd dashboard/stats/SSE routes, askama templates (XSS), non-macOS codepaths.** Not audited line-by-line this pass. Next baseline should expand scope.

## Gate evaluation

Per the `security-audit-baseline` capability spec: **unresolved `Fix now` CRITICAL/HIGH findings block archive**. This baseline has **0 unresolved Fix-now items** — all four applied and committed in this change. The 6 Triaged HIGH items are permissible per the spec provided their ROADMAP bullets exist (see `docs/ROADMAP.md`).

This baseline is **archivable**.
