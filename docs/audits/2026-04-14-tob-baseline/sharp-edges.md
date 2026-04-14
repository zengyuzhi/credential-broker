# Sharp-Edges Audit — credential-broker

**Provenance**
| Field | Value |
|-------|-------|
| Audit date | 2026-04-14 |
| Skill | Trail of Bits `sharp-edges` v1.0.0 |
| Auditor | Claude Sonnet 4.6 (automated subagent) |
| Scope | `/Users/zengy/credential-broker` (full workspace) |
| Methodology | "Pit of success" — identify APIs/configs where the easy path leads to insecurity |
| Read-only | Yes — no source changes made |

---

## Finding Summary

| Severity | Count |
|----------|-------|
| CRITICAL | 1 |
| HIGH | 4 |
| MEDIUM | 4 |
| LOW | 3 |
| INFO | 2 |
| **Total** | **14** |

---

## Findings

### SE-01 — Non-Constant-Time PIN Hash Comparison
**Severity:** CRITICAL
**File:** `crates/vaultd/src/auth.rs:229`
**Category:** Silent Failure / Timing Side-Channel

**Description:**
The PIN verification path compares the blake3 hex digest of the submitted PIN against the stored hash using the standard Rust `!=` operator on `String` values. This is a textbook timing oracle: an attacker who can submit many PIN attempts and measure response time can learn whether the first N bytes of the hash match, enabling a statistical brute-force attack without triggering the attempt-count lockout.

```rust
// auth.rs line 229
if hash(&body.pin) != session.pin_hash {
```

**Why it's a sharp edge:**
Using `!=` on `String` is the obvious, natural Rust comparison. Nothing in the type system or compiler warns that this is timing-unsafe for security comparisons. The developer must proactively know to use a constant-time comparator.

**Pit-of-success failure:**
The easy path (`!=`) is insecure. The secure path requires importing `subtle::ConstantTimeEq` or a similar crate, which is non-obvious.

**Recommended fix:**
Replace with constant-time comparison using the `subtle` crate:
```rust
use subtle::ConstantTimeEq;
if hash(&body.pin).as_bytes().ct_eq(session.pin_hash.as_bytes()).unwrap_u8() == 0 {
```

---

### SE-02 — Non-Constant-Time CSRF Token Comparison
**Severity:** HIGH
**File:** `crates/vaultd/src/auth.rs:355`
**Category:** Silent Failure / Timing Side-Channel

**Description:**
CSRF token validation uses standard string equality (`!=`), making CSRF tokens susceptible to the same timing oracle as the PIN (SE-01). An attacker able to make cross-origin requests (same-host, different context) could time-attack the CSRF token.

```rust
// auth.rs line 355
if csrf_header_value != expected || expected.is_empty() {
```

**Additional concern:** The `expected.is_empty()` guard is checked AFTER the non-constant-time comparison, meaning an empty expected value short-circuits but only after the timing-leaking comparison runs.

**Recommended fix:** Same as SE-01 — use `subtle::ConstantTimeEq`.

---

### SE-03 — Rate-Limit Key Derived from Spoofable HTTP Header
**Severity:** HIGH
**File:** `crates/vaultd/src/auth.rs:132-137`
**Category:** Configuration Cliff / Stringly-Typed Security

**Description:**
The PIN-attempt rate limiter keys on the `x-forwarded-for` header, falling back to `host`, falling back to the string literal `"local"`. Any local process (or proxied request) can set `x-forwarded-for` to an arbitrary string, trivially defeating the rate limit by rotating the header value on each attempt.

```rust
let caller_key = headers
    .get("x-forwarded-for")
    .or_else(|| headers.get("host"))
    .and_then(|v| v.to_str().ok())
    .unwrap_or("local")
    .to_string();
```

**Sharp edge:** Trusting a client-supplied header for rate-limiting identity is dangerous by default. The "obvious" approach (use the client's identifier from the request) leads directly to bypass.

**Recommended fix:** Key the rate limiter on a server-controlled identifier (e.g., session ID, socket peer address) rather than a client-supplied header. Since vaultd binds to `127.0.0.1` only, `peer_addr` from the socket is appropriate.

---

### SE-04 — `SecretStore::put` Stores Secrets Without Keychain ACL
**Severity:** HIGH
**File:** `crates/vault-secrets/src/keychain.rs` (put method, ~line 137)
**Category:** Dangerous Default

**Description:**
The `SecretStore` trait exposes two storage methods:
- `put_with_access`: correctly calls `/usr/bin/security add-generic-password` with `-T` flags restricting which apps can read the secret
- `put`: calls `security_framework::passwords::set_generic_password` with NO ACL — any application on the system can read the secret without a Keychain prompt

The trait's `put` method is the simpler, default path. Any caller that reaches for `put` instead of `put_with_access` stores secrets without access control, silently. There is no compiler warning, no runtime warning, and no trait-level documentation distinguishing the two.

**Sharp edge:** The easy path (`put`) is insecure. The secure path (`put_with_access`) requires knowing it exists and deliberately choosing it.

**Recommended fix:**
- Remove or `#[deprecated]` the `put` method from `SecretStore`
- Make `put_with_access` the only write path, or rename it to `put` and rename the old `put` to `put_without_acl` with a `#[deprecated(note = "stores without ACL — use put() instead")]` attribute

---

### SE-05 — No Secret Zeroization (Workspace-Wide)
**Severity:** HIGH
**File:** All crates — `vault-secrets`, `vault-cli`, `vaultd`
**Category:** Silent Failure

**Description:**
The `zeroize` crate is not present in any `Cargo.toml` in the workspace. API key bytes loaded from the Keychain are stored in `String`/`Vec<u8>` values, passed through function call chains, placed into `HashMap<String, String>`, and injected into child process environments. At no point are these secret bytes explicitly zeroed before the memory is reclaimed.

In practice this means:
- Secrets may linger in process heap pages after their `String` is dropped
- On process crash, a core dump could expose raw API keys
- A memory-inspection attack on a long-lived `vaultd` process has a higher exposure window

**Sharp edge:** Rust's ownership model makes developers feel safe about cleanup, but `Drop` for `String`/`Vec` calls `dealloc` without zeroing. The developer must proactively opt in to zeroing — the easy path leaves secrets in memory.

**Recommended fix:** Add `zeroize` to `vault-secrets` and `vault-cli`. Wrap key material in `ZeroizeOnDrop`-annotated types or call `.zeroize()` explicitly before return. Consider `secrecy::Secret<String>` as a wrapper.

---

### SE-06 — Timestamp Overflow Falls Back to `now` (Silent Instant-Expiry Bypass)
**Severity:** MEDIUM
**File:** `crates/vaultd/src/auth.rs:157`, `crates/vault-db/src/ui_sessions.rs:~86`
**Category:** Dangerous Default / Algorithm Footgun

**Description:**
Two locations compute an expiry timestamp using `checked_add_signed(Duration::...)` and fall back to `Utc::now()` on overflow:

```rust
// auth.rs ~line 157
.checked_add_signed(chrono::Duration::minutes(5))
.unwrap_or_else(Utc::now)  // overflow → expiry = now → token immediately invalid

// ui_sessions.rs ~line 86
.checked_add_signed(Duration::hours(4))
.unwrap_or_else(Utc::now)
```

`unwrap_or(now)` is semantically "if the arithmetic overflows, set expiry to right now, making the session immediately invalid." This silently denies service rather than panicking. While not an immediate security escalation (sessions expire instantly, not never), the silent failure pattern is dangerous: if the fallback were `DateTime::MAX` (the other obvious choice), sessions would never expire.

**Sharp edge:** The correct fallback is ambiguous — `now` means "deny service silently," `MAX` means "never expire." Neither is clearly right, and the code chooses one silently with no documentation.

**Recommended fix:** Return a hard error from the expiry computation rather than silently substituting a fallback. Callers can handle the error explicitly.

---

### SE-07 — `ttl_minutes: i64` Unchecked in `issue_lease`
**Severity:** MEDIUM
**File:** `crates/vault-policy/src/lease.rs:19`
**Category:** Dangerous Default / Configuration Cliff

**Description:**
`issue_lease` accepts `ttl_minutes: i64` with no validation. Passing 0 produces a lease that is expired immediately upon creation. Passing a negative value may panic (debug) or produce undefined behavior (release) during `Duration::minutes()` construction. Passing an extremely large value could overflow the timestamp arithmetic.

```rust
pub fn issue_lease(profile_id: Uuid, agent_name: &str, project: Option<String>, ttl_minutes: i64) -> (Lease, String) {
    // ...
    expires_at: issued_at + Duration::minutes(ttl_minutes),  // no bounds check
```

Currently the only call site hard-codes `60`, so exploitation requires changing the call site. But the API itself is a footgun: any future caller can pass an unchecked value.

**Recommended fix:** Validate `ttl_minutes > 0 && ttl_minutes <= MAX_LEASE_MINUTES` at the start of `issue_lease`, returning a `Result`. Or accept a `NonZeroU32` to make zero impossible at the type level.

---

### SE-08 — `u128 as i64` Truncating Cast for Latency Telemetry
**Severity:** MEDIUM
**File:** `crates/vaultd/src/routes/proxy.rs:161`
**Category:** Algorithm Footgun / Primitive API

**Description:**
`Instant::elapsed().as_millis()` returns `u128`. Casting to `i64` truncates silently when the latency exceeds ~292 million years (irrelevant in practice), but more importantly: if for any reason `elapsed()` returns a very large value (monotonic clock jump, system sleep), the cast is a silent truncation rather than a saturating or checked operation. Additionally, the semantics of a negative `latency_ms` in the DB are undefined.

```rust
let latency_ms = start.elapsed().as_millis() as i64;  // proxy.rs line 161
```

**Recommended fix:** Use `u64::try_from(...).unwrap_or(i64::MAX as u64) as i64` or `saturating_cast`. Document that values > `i64::MAX` are clamped.

---

### SE-09 — `f64` for Monetary Accumulation (`estimated_cost_usd`)
**Severity:** MEDIUM
**File:** `crates/vault-core/src/models.rs` (`UsageEvent.estimated_cost_usd: Option<f64>`)
**Category:** Primitive vs. Semantic API

**Description:**
`estimated_cost_usd` is typed as `Option<f64>`. Accumulating floating-point dollars across many `UsageEvent` rows introduces rounding error. For example, summing 0.001 USD across 1000 events in f64 may not equal exactly 1.00. While this is a telemetry/stats field and not used for billing, the sharp edge is that `f64` is the obvious choice for "a number with cents" and most developers won't reach for a fixed-point type.

**Recommended fix:** Use `Option<i64>` with a fixed-point convention (e.g., microdollars = USD × 1_000_000) and document the unit. This is lossless and accumulates exactly.

---

### SE-10 — `secret_ref` Service Component Silently Discarded
**Severity:** LOW
**File:** `crates/vaultd/src/routes/proxy.rs:209`, `crates/vault-cli/src/commands/run.rs:227`
**Category:** Silent Failure

**Description:**
`secret_ref` has the format `<service>:<account>`, but in both the proxy route and the CLI run command, the service component is parsed out and then discarded (`let (_service, account) = ...`). The Keychain lookup then hard-codes `KEYCHAIN_SERVICE_NAME` as the service, ignoring whatever was stored in `secret_ref`. If a secret were stored under a different service name (e.g., by a future migration or a different tool), the lookup would silently succeed against the wrong item or fail without a clear error.

**Recommended fix:** Either validate that the parsed `service` matches `KEYCHAIN_SERVICE_NAME` and return an error if not, or remove the service prefix from `secret_ref` if it is always implicit.

---

### SE-11 — `VAULT_TRUSTED_APP_PATHS` Paths Passed Unsanitized as CLI Arguments
**Severity:** LOW
**File:** `crates/vault-secrets/src/access.rs`
**Category:** Stringly-Typed Security / Configuration Cliff

**Description:**
`trusted_application_paths_for` reads `VAULT_TRUSTED_APP_PATHS` (colon-separated paths) from the environment and passes each as a `-T <path>` argument to `/usr/bin/security add-generic-password`. Paths containing spaces or special characters (e.g., semicolons) are passed without escaping or quoting to `Command::args()`.

While `Command::args()` in Rust (via `execvp`) does not use a shell and therefore does not interpret shell metacharacters, paths with embedded null bytes or other unusual characters could still cause surprising behavior in the `security` CLI argument parser.

More relevantly, an attacker who can write `VAULT_TRUSTED_APP_PATHS` can inject paths to arbitrary binaries (e.g., `/tmp/malicious`) into the Keychain ACL, granting those binaries access to all vault-managed secrets.

**Recommended fix:** Document that `VAULT_TRUSTED_APP_PATHS` is a privileged configuration value that must not be user-controllable. Validate that each path is an absolute path and exists as a file before passing to `/usr/bin/security`.

---

### SE-12 — CSRF/Origin Extraction Defaults to Empty String
**Severity:** LOW
**File:** `crates/vaultd/src/auth.rs:339, 352, 354`
**Category:** Dangerous Default

**Description:**
Multiple `.unwrap_or("")` calls populate the CSRF origin and token variables before the security check:

```rust
// auth.rs ~line 339, 352, 354
let origin = headers.get("origin").and_then(|v| v.to_str().ok()).unwrap_or("");
let csrf_header_value = headers.get("x-csrf-token").and_then(|v| v.to_str().ok()).unwrap_or("");
if csrf_header_value != expected || expected.is_empty() { ... }
```

The `expected.is_empty()` guard prevents the empty-string case from bypassing the check (if `expected` is also `""`, the guard fires). However, the defaulting to `""` is confusing: a missing CSRF token and an empty-string CSRF token are treated identically, and the logic is non-obvious enough to invite future regression.

**Recommended fix:** Return `Err(UNAUTHORIZED)` early when `csrf_header_value` is absent (missing header), before reaching the comparison. Make absence and empty-string explicit different error conditions.

---

### SE-13 — Lease TTL Hardcoded at 60 Minutes (Not Configurable)
**Severity:** INFO
**File:** `crates/vault-cli/src/commands/run.rs:130`
**Category:** Configuration Cliff

**Description:**
```rust
let (lease, raw_token) = issue_lease(profile.id, &cmd.agent, cmd.project.clone(), 60);
```

The 60-minute TTL is hardcoded at the call site. There is no `--lease-ttl` flag on `vault run`. This means long-running agent processes get a non-revokable window of exactly 60 minutes regardless of sensitivity. There is no mechanism to shorten the window for high-sensitivity credentials.

**Note:** Low severity because the window is bounded (not infinite) and the current use case is local tooling.

---

### SE-14 — Widening `as i64` / `as u16` Casts in Telemetry
**Severity:** INFO
**File:** `crates/vault-cli/src/commands/run.rs:182`, `crates/vaultd/src/routes/proxy.rs:184`
**Category:** Primitive vs. Semantic API

**Description:**
```rust
status_code: status.code().map(|c| c as i64),  // run.rs — i32 → i64, safe widening
status_code: Some(status_code.as_u16() as i64), // proxy.rs — u16 → i64, safe widening
```

These casts are range-safe in practice (i32 fits in i64; u16 fits in i64). Noted for completeness: using `i64::from(c)` and `i64::from(status_code.as_u16())` would make the widening explicit and fail to compile if the source type ever changed to something wider.

---

## Scan Gaps

The following areas were not exhaustively audited at line-by-line granularity:

| Area | Gap |
|------|-----|
| `crates/vault-db/src/{credentials,profiles,bindings,leases,usage_events}.rs` | SQL query safety spot-checked (all use `sqlx::query!` macros with parameterized bindings — no string concatenation observed) but not read line-by-line |
| `crates/vault-providers/src/` | Provider adapters (OpenAI, Anthropic, TwitterAPI) not individually audited for injection or response-parsing sharp edges |
| `crates/vaultd/src/routes/{stats,events,dashboard}.rs` | Dashboard/SSE/stats routes not read |
| `crates/vaultd/templates/` | Askama templates not audited for XSS (Askama auto-escapes by default, but custom filters could bypass) |
| Non-macOS targets | All `#[cfg(not(target_os = "macos"))]` stubs produce `bail!` errors — not audited for logic in future cross-platform expansion |
| `Cargo.lock` dependency versions | Not audited for known CVEs (separate supply-chain audit in progress) |

---

## Methodology Notes

This audit applied the Trail of Bits Sharp-Edges framework, focusing on:
1. **Dangerous defaults** — zero/empty/null edge cases, insecure default paths
2. **Algorithm selection footguns** — where developers choose primitives and can choose wrong
3. **Silent failures** — security checks that return false/empty instead of erroring
4. **Stringly-typed security** — permission strings, config values, header-derived identity
5. **Configuration cliffs** — one wrong value breaks security silently

The three adversary models evaluated were: the Scoundrel (actively malicious config), the Lazy Developer (copy-paste usage), and the Confused Developer (API misuse).
