# CLAUDE.md
You are being supervised by competing AI models and domain experts. Desgin code and documentation with the expectation that it will be scrutinized by other models and experts in Rust, security, and credential management. Write clear, maintainable code with comprehensive comments and documentation. Anticipate potential questions or critiques and address them proactively in the code and docs.

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```bash
cargo build                          # build all crates
cargo test                           # run all tests
cargo clippy --workspace --all-targets -- -D warnings  # lint (strict)
cargo run -p vault-cli -- <subcmd>   # run CLI (e.g. credential list)
vault serve                          # start server on 127.0.0.1:8765
vault serve --background             # start in background with PID file
vault serve stop                     # stop background server
vault ui                             # auto-start server + open dashboard
cargo run -p vaultd                  # deprecated, use vault serve instead
cargo test -p vault-db               # test a single crate
cargo fmt --all                       # format all crates
```

The SQLite database lives at `.local/vault.db` (gitignored). Override with `VAULT_DATABASE_URL` env var. Migrations in `migrations/` are auto-applied by `vault-db` on pool creation via `sqlx::migrate!`.

## Architecture

This is a local credential broker that lets coding agents and scripts access API keys without seeing the raw secrets. Two binaries, six library crates (`vault serve` in vault-cli replaces standalone vaultd):

```
vault-cli (binary)          vaultd (binary)
    │                           │
    ├── vault-policy            ├── vault-policy
    ├── vault-telemetry         │
    ├── vault-providers         ├── vault-providers
    ├── vault-secrets           ├── vault-secrets
    ├── vault-db                ├── vault-db
    └── vault-core ─────────────┘
```

### Crate responsibilities

- **vault-core** — Domain types (`Credential`, `Profile`, `Lease`, `UsageEvent`), the `ProviderAdapter` async trait, and `VaultError`. No internal deps — everything depends on this.
- **vault-db** — `Store` wraps `SqlitePool`. Sub-modules (`credentials`, `profiles`, `bindings`, `leases`, `usage_events`) each implement CRUD. All queries use manual `map_*_row` functions (not `FromRow` derive).
- **vault-secrets** — `SecretStore` trait with a macOS Keychain implementation (`security-framework`). Secrets stored under service `ai.zyr1.vault` with ref format `<service>:<credential_id>:<field_name>`.
- **vault-providers** — `ProviderAdapter` implementations (OpenAI, Anthropic, TwitterAPI). `registry::adapter_for()` returns the right adapter. `schema.rs` has static `ProviderSchema` definitions for 7 providers (only 3 have full adapters).
- **vault-policy** — Lease issuance (`issue_lease` generates UUID token, blake3-hashes it) and `PolicyService` (blocks prod credentials unless `allow_prod` is set).
- **vault-telemetry** — `TelemetryWriter` persists `UsageEvent` rows via `Store`. `StatsSummary` struct for rollups.
- **vault-cli** — Clap-derived CLI. Fully working: `credential add/list/enable/disable/remove`, `profile create/list/show/bind`, `run --profile <name> -- <cmd>`, `stats`. Records launch events via telemetry.
- **vaultd** — Axum HTTP daemon. Routes: `GET /health`, `GET /stats/providers` (real rollup data), `POST /v1/proxy/{provider}/{*path}` (lease-authenticated upstream forwarding).

### Key data flow: `vault run`

1. CLI resolves profile name → loads profile bindings from DB
2. For each binding, reads secret from macOS Keychain via `SecretStore`
3. Provider adapter's `env_map()` builds env vars (e.g. `OPENAI_API_KEY`)
4. `issue_lease()` generates a session token (raw + blake3 hash stored in DB)
5. Injects `VAULT_PROFILE`, `VAULT_AGENT`, `VAULT_LEASE_TOKEN`, `VAULT_PROJECT` into env
6. Spawns child process with the combined environment

### Access modes

Credentials bind to profiles with a `mode`: `Inject` (env vars), `Proxy` (vaultd forwards requests), or `Either`. Both Inject and Proxy are implemented.

### Web dashboard

Served by vaultd (embedded in vault-cli via `vault serve`). Stack: askama templates + Pico CSS + htmx (CDN). No npm/node.
- Auth: daemon-owned PIN challenges (blake3 hashed, 5-attempt burn, 5-min expiry, 3/min rate limit)
- CSRF: per-session token in `<meta>` tag, sent via `htmx:configRequest` header
- SSE: SQLite polling every 2s at `GET /api/events` for cross-process change detection
- Templates: `crates/vaultd/templates/*.html` — use `tmpl.render()` match pattern, NOT `IntoResponse` trait
- Background: `vault serve --background` re-execs with `process_group(0)`, PID file at `.local/vault.pid`

## Conventions

- Rust edition 2024, stable toolchain
- All IDs are UUID v4 strings (not native UUID columns — SQLite stores them as TEXT)
- Timestamps are ISO 8601 strings in SQLite (`chrono` for serialization)
- Errors: `VaultError` (thiserror) for domain errors, `anyhow::Result` for plumbing
- macOS-only for now: `vault-secrets` uses `security-framework` behind `#[cfg(target_os = "macos")]`
- DB queries: use manual `map_*_row(SqliteRow) -> Result<T>` functions with `sqlx::Row::get()`, NOT `#[derive(FromRow)]`. Codec helpers: `access_mode_as_str()` / `parse_access_mode()`.

## Gotchas

- `askama_axum` 0.4 `IntoResponse` impl is incompatible with axum 0.8 — always use manual `tmpl.render()` → `Html(html).into_response()`.
- Background process detach on macOS: use `cmd.process_group(0)` (`std::os::unix::process::CommandExt`).
- PID file at `.local/vault.pid` — always check for stale PIDs (process dead but file exists).
- `security-framework` v3 does NOT bind Keychain ACL APIs (`SecAccessCreate`, `SecTrustedApplicationCreateFromPath`). Trusted-app ACLs use `/usr/bin/security` CLI — always absolute path, never bare `security`.
- macOS `security add-generic-password`: place `-w` as the **last** argument with no value to read the password from stdin (avoids process-list exposure).
- `MacOsKeychainStore` is a unit struct — construct with `MacOsKeychainStore`, not `MacOsKeychainStore::default()` (clippy `default_constructed_unit_structs`).
- Tests use `std::sync::Mutex` for DB URL serialization across async tests — annotate with `#[allow(clippy::await_holding_lock)]`.
- `zeroize` 1.x has no blanket `Zeroize` impl for `HashMap<K, V>` — `#[derive(ZeroizeOnDrop)]` on a struct carrying `HashMap<String, String>` fails to compile. Use a manual `impl Drop` iterating `values_mut().zeroize()` (see `vault-core/src/provider.rs`).
- `Zeroizing<String>` derefs to `&str` for most call sites, but `HeaderValue::from(&secret)` fails with `From<&Zeroizing<String>>` unimplemented — pass `secret.as_str()` explicitly at HTTP-header boundaries.
- `SecretStore::get` returns `Zeroizing<String>`, not `String`. Callers that need a plain `String` clone via `(*value).clone()` so the new copy still drops into whatever wipe-on-drop wrapper its destination uses.
- `vault_policy::lease::issue_lease` takes `ttl_minutes: std::num::NonZeroU32`, not `i64`. Compile-time constants use `NonZeroU32::new(60).expect("60 is nonzero")`; CLI-parsed input must be validated at the parser boundary.
- Monetary cost split brain: `UsageEvent.estimated_cost_micros: Option<i64>` is the internal/DB type; external JSON (`/v1/stats/providers`, `ProviderStats`) preserves the `estimated_cost_usd: f64` field name for backward compat via `CAST(SUM(...) AS REAL) / 1000000.0` at the SQL boundary.
- DB backup convention before destructive migrations: `cp .local/vault.db .local/vault.db.pre-<NNNN>.bak`. `.local/` is gitignored so backups stay local.
- Implementation plans live in `docs/plans/` — check there before starting new work.

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **credential-broker** (2362 symbols, 3938 relationships, 198 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> If any GitNexus tool warns the index is stale, run `npx gitnexus analyze` in terminal first.

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `gitnexus_impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `gitnexus_detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `gitnexus_query({query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `gitnexus_context({name: "symbolName"})`.

## When Debugging

1. `gitnexus_query({query: "<error or symptom>"})` — find execution flows related to the issue
2. `gitnexus_context({name: "<suspect function>"})` — see all callers, callees, and process participation
3. `READ gitnexus://repo/credential-broker/process/{processName}` — trace the full execution flow step by step
4. For regressions: `gitnexus_detect_changes({scope: "compare", base_ref: "main"})` — see what your branch changed

## When Refactoring

- **Renaming**: MUST use `gitnexus_rename({symbol_name: "old", new_name: "new", dry_run: true})` first. Review the preview — graph edits are safe, text_search edits need manual review. Then run with `dry_run: false`.
- **Extracting/Splitting**: MUST run `gitnexus_context({name: "target"})` to see all incoming/outgoing refs, then `gitnexus_impact({target: "target", direction: "upstream"})` to find all external callers before moving code.
- After any refactor: run `gitnexus_detect_changes({scope: "all"})` to verify only expected files changed.

## Never Do

- NEVER edit a function, class, or method without first running `gitnexus_impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `gitnexus_rename` which understands the call graph.
- NEVER commit changes without running `gitnexus_detect_changes()` to check affected scope.

## Tools Quick Reference

| Tool | When to use | Command |
|------|-------------|---------|
| `query` | Find code by concept | `gitnexus_query({query: "auth validation"})` |
| `context` | 360-degree view of one symbol | `gitnexus_context({name: "validateUser"})` |
| `impact` | Blast radius before editing | `gitnexus_impact({target: "X", direction: "upstream"})` |
| `detect_changes` | Pre-commit scope check | `gitnexus_detect_changes({scope: "staged"})` |
| `rename` | Safe multi-file rename | `gitnexus_rename({symbol_name: "old", new_name: "new", dry_run: true})` |
| `cypher` | Custom graph queries | `gitnexus_cypher({query: "MATCH ..."})` |

## Impact Risk Levels

| Depth | Meaning | Action |
|-------|---------|--------|
| d=1 | WILL BREAK — direct callers/importers | MUST update these |
| d=2 | LIKELY AFFECTED — indirect deps | Should test |
| d=3 | MAY NEED TESTING — transitive | Test if critical path |

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/credential-broker/context` | Codebase overview, check index freshness |
| `gitnexus://repo/credential-broker/clusters` | All functional areas |
| `gitnexus://repo/credential-broker/processes` | All execution flows |
| `gitnexus://repo/credential-broker/process/{name}` | Step-by-step execution trace |

## Self-Check Before Finishing

Before completing any code modification task, verify:
1. `gitnexus_impact` was run for all modified symbols
2. No HIGH/CRITICAL risk warnings were ignored
3. `gitnexus_detect_changes()` confirms changes match expected scope
4. All d=1 (WILL BREAK) dependents were updated

## Keeping the Index Fresh

After committing code changes, the GitNexus index becomes stale. Re-run analyze to update it:

```bash
npx gitnexus analyze
```

If the index previously included embeddings, preserve them by adding `--embeddings`:

```bash
npx gitnexus analyze --embeddings
```

To check whether embeddings exist, inspect `.gitnexus/meta.json` — the `stats.embeddings` field shows the count (0 means no embeddings). **Running analyze without `--embeddings` will delete any previously generated embeddings.**

> Claude Code users: A PostToolUse hook handles this automatically after `git commit` and `git merge`.

## CLI

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->
