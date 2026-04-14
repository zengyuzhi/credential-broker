# Zeroize Audit — credential-broker

## Provenance

| Field | Value |
|---|---|
| Skill | `zeroize-audit@trailofbits/skills` |
| Invocation date | `2026-04-14` |
| Scope | `/Users/zengy/credential-broker` (workspace root) |
| Focused paths | `crates/vault-secrets/`, `crates/vault-cli/src/commands/run.rs`, `crates/vault-cli/src/commands/serve.rs`, `crates/vaultd/src/routes/proxy.rs`, `crates/vault-policy/` |
| Tool version | `zeroize-audit 0.1.0` (skill base: `trailofbits/zeroize-audit`) |
| Serena MCP | Available — used for semantic symbol resolution |
| IR/ASM analysis | Skipped — `cargo build` excluded per audit boundaries; no `compile_commands.json` provided |
| Confidence gate | Source-only analysis; IR/ASM-required findings (`OPTIMIZED_AWAY_ZEROIZE`, `STACK_RETENTION`, `REGISTER_SPILL`) emitted as `needs_review` pending compiler evidence |
| Scan completeness | Phase 1 (source analysis) complete. Phase 2 (compiler/IR/ASM) skipped per scope boundary. |

---

## Sensitive Objects Inventory

| ID | Type | Symbol | Location | Notes |
|---|---|---|---|---|
| SO-5000 | `Vec<u8>` → `String` | `bytes` / `secret` in `MacOsKeychainStore::get` | `crates/vault-secrets/src/keychain.rs:146-148` | Raw Keychain bytes converted to `String`; no zeroize on drop |
| SO-5001 | `&str` → stdin bytes | `secret` in `MacOsKeychainStore::put_with_access` | `crates/vault-secrets/src/keychain.rs:83-132` | Secret piped to subprocess stdin; async write buffer not wiped |
| SO-5002 | `String` | `secret` in `load_secret` (proxy) | `crates/vaultd/src/routes/proxy.rs:214` | API key returned as owned `String`; lives until request scope ends, no zeroize |
| SO-5003 | `String` | `secret` in `proxy_handler` | `crates/vaultd/src/routes/proxy.rs:121-143` | Same `String` injected as HTTP header value into `reqwest` request builder; no wipe after send |
| SO-5004 | `String` | `raw_token` in `issue_lease` | `crates/vault-policy/src/lease.rs:11-22` | 64-hex-char raw token (two UUIDs concatenated); passed to caller, then inserted into child env; no zeroize |
| SO-5005 | `HashMap<String,String>` | `env_map` in `run_agent_command` | `crates/vault-cli/src/commands/run.rs:125-154` | Contains all API keys + `VAULT_LEASE_TOKEN`; dropped after `.status().await`; no zeroize on map values |
| SO-5006 | `String` | `raw_token` in `login_handler` (vaultd auth) | `crates/vaultd/src/auth.rs:246-263` | Dashboard session token placed directly into `set-cookie` header string; not wiped |
| SO-5007 | `String` | `pin` in `challenge_handler` | `crates/vaultd/src/auth.rs:147-183` | 6-digit PIN generated and returned in JSON response body; plain `String`, no wipe |
| SO-5008 | `String` | `secret_value` in `resolve_bound_credentials` | `crates/vault-cli/src/commands/run.rs:234-256` | Keychain API key placed into `ResolvedCredential.fields` HashMap; propagates through `resolve_env_for_profile` without zeroize |
| SO-5009 | `Vec<u8>` | `bytes` returned by `get_generic_password` | `crates/vault-secrets/src/keychain.rs:146` | `security_framework` returns an owned `Vec<u8>`; consumed by `String::from_utf8(bytes)` which moves (does not zero) the buffer |

---

## Findings

### ZA-0001 — MISSING_SOURCE_ZEROIZE — `Vec<u8>` keychain buffer not wiped after conversion

| Field | Value |
|---|---|
| **ID** | ZA-0001 |
| **Category** | `MISSING_SOURCE_ZEROIZE` |
| **Severity** | HIGH |
| **Confidence** | likely |
| **Language** | Rust |
| **File** | `crates/vault-secrets/src/keychain.rs` |
| **Lines** | 146–148 |
| **Symbol** | `MacOsKeychainStore::get` |
| **Sensitive object** | SO-5000, SO-5009 |

**Evidence (source):**

```rust
let bytes = get_generic_password(service, account)
    .with_context(|| format!("failed to load secret for {service}/{account}"))?;
let secret = String::from_utf8(bytes).context("keychain secret is not valid utf-8")?;
Ok(secret)
```

`get_generic_password` returns `Result<Vec<u8>>`. The `Vec<u8>` is consumed by `String::from_utf8(bytes)`, which moves the internal heap buffer ownership into the new `String` — it does not zero the original allocation. The returned `String` (SO-5000) then propagates up to `resolve_bound_credentials` → `resolve_env_for_profile` → `env_map` → child process env. When the `env_map` `HashMap<String,String>` is dropped after `.status().await`, Rust's default `Drop` for `String` simply calls `dealloc` without zeroing. The API key bytes remain readable in the heap until the allocator reuses the page.

No `zeroize` crate is listed in any workspace `Cargo.toml`. No `Zeroizing<T>` wrapper, `ZeroizeOnDrop`, or manual volatile-wipe is present anywhere in the codebase.

**IR/ASM note:** Assembly confirmation that the dealloc path skips zeroing was not obtained (build excluded per scope). Finding is `likely` pending IR diff.

**Suggested fix:**
```rust
use zeroize::Zeroizing;

async fn get(&self, service: &str, account: &str) -> anyhow::Result<String> {
    let _lock = SecKeychain::disable_user_interaction()...;
    let bytes: Zeroizing<Vec<u8>> = Zeroizing::new(
        get_generic_password(service, account)
            .with_context(|| ...)?
    );
    let secret = Zeroizing::new(
        String::from_utf8(bytes.to_vec()).context("keychain secret is not valid utf-8")?
    );
    Ok(secret.into_inner()) // caller must also wrap in Zeroizing
}
```
Alternatively, wrap the final `String` in `secrecy::Secret<String>` and propagate a secret-aware type through the call chain.

---

### ZA-0002 — MISSING_SOURCE_ZEROIZE — API key `String` persists in `env_map` after child spawn

| Field | Value |
|---|---|
| **ID** | ZA-0002 |
| **Category** | `MISSING_SOURCE_ZEROIZE` |
| **Severity** | HIGH |
| **Confidence** | likely |
| **Language** | Rust |
| **File** | `crates/vault-cli/src/commands/run.rs` |
| **Lines** | 125–161 |
| **Symbol** | `run_agent_command` |
| **Sensitive objects** | SO-5004, SO-5005, SO-5008 |

**Evidence (source):**

```rust
let mut env_map = resolve_env_for_profile(resolved)?;
let (lease, raw_token) = issue_lease(...);
env_map.insert("VAULT_LEASE_TOKEN".to_string(), raw_token);
// ...
let status = Command::new(&program)
    .envs(&env_map)
    .status()
    .await?;
// env_map drops here — no zeroize
```

The `env_map: HashMap<String, String>` holds every bound API key (e.g., `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`) plus `VAULT_LEASE_TOKEN` (the raw 64-character lease token). After `.status().await` returns, `env_map` is dropped by Rust's default allocator path — no zeroing occurs. All secret values remain live on the heap until reuse.

Additionally, `raw_token` from `issue_lease` (SO-5004) is a `String` built from two `Uuid::new_v4()` values concatenated via `format!`. This token is inserted into `env_map` and never independently wiped.

**Suggested fix:** Wrap `env_map` values in `Zeroizing<String>` or use `secrecy::SecretString`. After spawning, explicitly clear each value with `zeroize::Zeroize::zeroize(&mut val)` before drop.

---

### ZA-0003 — MISSING_SOURCE_ZEROIZE — API key `String` injected into HTTP request header without wipe

| Field | Value |
|---|---|
| **ID** | ZA-0003 |
| **Category** | `MISSING_SOURCE_ZEROIZE` |
| **Severity** | HIGH |
| **Confidence** | likely |
| **Language** | Rust |
| **File** | `crates/vaultd/src/routes/proxy.rs` |
| **Lines** | 121–143 |
| **Symbol** | `proxy_handler` |
| **Sensitive objects** | SO-5002, SO-5003 |

**Evidence (source):**

```rust
let secret = load_secret(&credential.secret_ref).await...?;
// ...
req_builder = match provider.as_str() {
    "anthropic" => req_builder.header("x-api-key", &secret),
    _ => req_builder.header("authorization", format!("Bearer {secret}")),
};
let upstream_response = req_builder.send().await...?;
// secret drops here — no zeroize
```

`secret` is an owned `String` constructed in `load_secret` → `MacOsKeychainStore::get`. It is passed by reference into the `reqwest` `RequestBuilder` which internally clones/copies the value into its header map. When the local `secret` goes out of scope at function return, it is dropped without zeroing. The `reqwest` internal copy (inside the builder and then the sent request's header buffer) also has no zeroize path. The `format!("Bearer {secret}")` branch additionally creates a second heap allocation containing the full key.

**Suggested fix:** Use `Zeroizing<String>` for `secret`. After `req_builder.send()`, there is no practical way to wipe the `reqwest` internal copy; the real fix is to avoid constructing the full key string in process memory — use `reqwest`'s streaming/header APIs with a `SecretString` wrapper and ensure the string is overwritten before drop.

---

### ZA-0004 — MISSING_SOURCE_ZEROIZE — `raw_token` in `issue_lease` not wiped after hash

| Field | Value |
|---|---|
| **ID** | ZA-0004 |
| **Category** | `MISSING_SOURCE_ZEROIZE` |
| **Severity** | MEDIUM |
| **Confidence** | likely |
| **Language** | Rust |
| **File** | `crates/vault-policy/src/lease.rs` |
| **Lines** | 5–22 |
| **Symbol** | `issue_lease` |
| **Sensitive object** | SO-5004 |

**Evidence (source):**

```rust
pub fn issue_lease(...) -> (Lease, String) {
    let raw_token = format!("{}{}", Uuid::new_v4(), Uuid::new_v4());
    let lease = Lease {
        ...
        session_token_hash: hash_token(&raw_token),
    };
    (lease, raw_token)
}
```

`raw_token` is a 64-character hex `String` that serves as the bearer credential for the lease. Its blake3 hash is stored; the raw token is returned to the caller and passed into `env_map` (see ZA-0002). `issue_lease` itself does not wipe `raw_token` prior to returning, leaving the pre-hash material on the heap. The caller in `run_agent_command` moves `raw_token` into `env_map` without wrapping it in a zeroizing type.

**Suggested fix:** Return `Zeroizing<String>` from `issue_lease`. The hash is derived first, so raw token material can be wiped immediately after hashing if the caller uses `Zeroizing`.

---

### ZA-0005 — MISSING_SOURCE_ZEROIZE — Dashboard session `raw_token` in `login_handler` not wiped

| Field | Value |
|---|---|
| **ID** | ZA-0005 |
| **Category** | `MISSING_SOURCE_ZEROIZE` |
| **Severity** | MEDIUM |
| **Confidence** | likely |
| **Language** | Rust |
| **File** | `crates/vaultd/src/auth.rs` |
| **Lines** | 246–263 |
| **Symbol** | `login_handler` |
| **Sensitive object** | SO-5006 |

**Evidence (source):**

```rust
let raw_token = Uuid::new_v4().to_string();
let session_token_hash = hash(&raw_token);
// ...
let cookie = set_session_cookie(&raw_token);
// raw_token drops here — no zeroize
```

`raw_token` is a UUID-derived session token embedded in the `set-cookie` response header. `set_session_cookie` formats it into a cookie string (another heap allocation). The original `raw_token` `String` is dropped without zeroing after the cookie header is built. The cookie `String` is similarly dropped without zeroing when the response is serialized.

**Suggested fix:** Wrap `raw_token` in `Zeroizing<String>`.

---

### ZA-0006 — SECRET_COPY — `secret_value` copied through `ResolvedCredential.fields` without tracking

| Field | Value |
|---|---|
| **ID** | ZA-0006 |
| **Category** | `SECRET_COPY` |
| **Severity** | MEDIUM |
| **Confidence** | needs_review (MCP available; cross-file data-flow confirmed by source read) |
| **Language** | Rust |
| **File** | `crates/vault-cli/src/commands/run.rs` |
| **Lines** | 264–273 |
| **Symbol** | `resolve_bound_credentials` |
| **Sensitive object** | SO-5008 |

**Evidence (source):**

```rust
resolved.push((
    credential.provider.clone(),
    binding.mode,
    ResolvedCredential {
        provider: credential.provider,
        label: credential.label,
        fields: HashMap::from([(field_name.to_string(), secret_value)]),
    },
));
```

`secret_value` (the raw API key `String` from the Keychain) is moved into `ResolvedCredential.fields`. The `ResolvedCredential` struct is defined in `vault-core` as a plain `struct` with no `Drop` customization or `Zeroize` derive. The value then flows through `resolve_env_for_profile` → `merge_env_map` → `env_map`. At each step the `String` is cloned or moved without zeroization tracking.

**Suggested fix:** Add `#[derive(Zeroize, ZeroizeOnDrop)]` to `ResolvedCredential` and use `Zeroizing<String>` for field values, or replace `HashMap<String, String>` with `HashMap<String, Zeroizing<String>>`.

---

### ZA-0007 — MISSING_SOURCE_ZEROIZE — `put_with_access` stdin buffer not explicitly wiped

| Field | Value |
|---|---|
| **ID** | ZA-0007 |
| **Category** | `MISSING_SOURCE_ZEROIZE` |
| **Severity** | LOW |
| **Confidence** | needs_review |
| **Language** | Rust |
| **File** | `crates/vault-secrets/src/keychain.rs` |
| **Lines** | 113–118 |
| **Symbol** | `MacOsKeychainStore::put_with_access` |
| **Sensitive object** | SO-5001 |

**Evidence (source):**

```rust
if let Some(mut stdin) = child.stdin.take() {
    stdin.write_all(secret.as_bytes()).await?;
    stdin.shutdown().await.ok();
}
```

`secret.as_bytes()` produces a slice into the caller-owned `&str`. The `tokio::process::ChildStdin::write_all` internally buffers the bytes before syscall. Tokio's I/O buffer is heap-allocated and will be dropped (not zeroed) when `stdin` goes out of scope at the end of the `if let` block. This is a secondary copy of the secret in process memory.

The `secret` parameter itself (`&str`) is borrowed from the caller — the caller is responsible for zeroizing the original. The `put_with_access` API contract does not communicate this requirement.

**Suggested fix:** Accept `secret: &Zeroizing<String>` or `secret: &SecretString` to make the contract explicit. Manually zero the I/O buffer slice post-write if Tokio's buffer is accessible, or use an OS pipe that flushes before the buffer is freed.

---

### ZA-0008 — INFO — `debug_log` via `VAULT_DEBUG_RUN` may expose secrets in `account` names

| Field | Value |
|---|---|
| **ID** | ZA-0008 |
| **Category** | `INFO` |
| **Severity** | INFO |
| **Confidence** | confirmed |
| **Language** | Rust |
| **File** | `crates/vault-cli/src/commands/run.rs` |
| **Lines** | 228–236 |
| **Symbol** | `resolve_bound_credentials` |

**Evidence (source):**

```rust
debug_log(format!(
    "resolving binding id={} provider={} credential_id={} mode={:?} account={}",
    binding.id, binding.provider, credential.id, binding.mode, account
));
// ...
debug_log(format!(
    "reading macOS keychain service={} account={}",
    KEYCHAIN_SERVICE_NAME, account
));
```

When `VAULT_DEBUG_RUN=1` is set, the Keychain `account` string (which is `<credential_id>:<field_name>` per the `secret_ref` format) is emitted to stderr. While this does not directly expose the secret value itself, it does expose the full credential identifier. If a `secret_ref` is ever constructed in a non-standard way that embeds partial key material, or if `account` is misinterpreted, this could constitute a metadata leak. No secret _values_ were found in `debug_log` calls.

**Suggested fix:** No immediate code change required. Consider gating debug output behind a build feature flag rather than a runtime env var to reduce accidental exposure in production environments.

---

### ZA-0009 — INFO — No `zeroize` crate dependency in any workspace member

| Field | Value |
|---|---|
| **ID** | ZA-0009 |
| **Category** | `INFO` |
| **Severity** | INFO |
| **Confidence** | confirmed |

**Evidence:** Grep of all workspace `Cargo.toml` files found zero references to `zeroize`, `secrecy`, or `memsec`. The `Cargo.lock` shows `zeroize` only as a transitive dependency of third-party crates (e.g., `ring`, `p256`), not as a direct dependency of any first-party crate.

This means there is no workspace-wide policy for secret handling types. All secrets are plain `String` or `Vec<u8>` values with standard (non-zeroing) drop semantics.

**Suggested fix:** Add `zeroize = { version = "1", features = ["derive"] }` to the workspace `[dependencies]` table and adopt `Zeroizing<T>` for all secrets across the codebase.

---

## Scan Gaps

1. **IR/ASM analysis not performed.** The skill's Phase 2 (compiler evidence for `OPTIMIZED_AWAY_ZEROIZE`, `STACK_RETENTION`, `REGISTER_SPILL`) was excluded per audit boundary ("do not run cargo build/test"). All source-level HIGH/MEDIUM findings above are `likely` rather than `confirmed` pending assembly corroboration.

2. **`security-framework` internal buffer.** The `get_generic_password` FFI call allocates memory inside the macOS Security framework (Objective-C runtime). Whether `security-framework-sys` calls `SecKeychainItemFreeContent` which zeros the buffer before freeing is outside the scope of Rust source analysis. This should be verified against the `security-framework` crate source and Apple's documentation.

3. **`reqwest` internal header buffers.** Once the API key is passed into `reqwest`'s `RequestBuilder::header`, it is copied into an internal `HeaderMap`. Whether `reqwest`/`hyper` zeros these buffers on drop is not audited here.

4. **Telemetry path.** `vault_telemetry::writer::TelemetryWriter` was not in scope for this audit. If telemetry serialization includes credential fields or error text containing secret material, that would be an additional finding.

---

## Preliminary Findings

| ID | Severity | Category | Location | Summary |
|---|---|---|---|---|
| ZA-0001 | **HIGH** | `MISSING_SOURCE_ZEROIZE` | `vault-secrets/src/keychain.rs:146-148` | `Vec<u8>` returned from Keychain FFI converted to `String` without zeroize; API key persists on heap after drop |
| ZA-0002 | **HIGH** | `MISSING_SOURCE_ZEROIZE` | `vault-cli/src/commands/run.rs:125-161` | `env_map` HashMap holding all API keys + lease token dropped without zeroize after child spawn |
| ZA-0003 | **HIGH** | `MISSING_SOURCE_ZEROIZE` | `vaultd/src/routes/proxy.rs:121-143` | API key `String` injected into HTTP `Authorization`/`x-api-key` header without wipe; secondary `format!("Bearer {secret}")` allocation also not wiped |
| ZA-0004 | **MEDIUM** | `MISSING_SOURCE_ZEROIZE` | `vault-policy/src/lease.rs:11-22` | Raw 64-char lease token not wiped after blake3 hashing; returned to caller as plain `String` |
| ZA-0005 | **MEDIUM** | `MISSING_SOURCE_ZEROIZE` | `vaultd/src/auth.rs:246-263` | Dashboard session `raw_token` dropped without zeroize after cookie string construction |
| ZA-0006 | **MEDIUM** | `SECRET_COPY` | `vault-cli/src/commands/run.rs:264-273` | API key `String` copied into `ResolvedCredential.fields` HashMap — no `Zeroize` derive on struct; multi-hop copy chain without tracking |
| ZA-0007 | **LOW** | `MISSING_SOURCE_ZEROIZE` | `vault-secrets/src/keychain.rs:113-118` | Tokio stdin write buffer for subprocess secret-ingest not zeroed on drop |
| ZA-0008 | **INFO** | Observation | `vault-cli/src/commands/run.rs:228-236` | `VAULT_DEBUG_RUN` debug path emits Keychain account names to stderr; no raw secret values observed |
| ZA-0009 | **INFO** | Observation | workspace `Cargo.toml` | Zero direct `zeroize` dependencies; no workspace-wide secure secret type policy |

**Finding counts by severity:** CRITICAL: 0 — HIGH: 3 — MEDIUM: 3 — LOW: 1 — INFO: 2
