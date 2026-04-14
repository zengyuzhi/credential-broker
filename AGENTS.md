# AGENTS.md - credential-broker

You are working in `credential-broker`, a local credential broker for coding agents and scripts.
Write code and docs assuming strong scrutiny from Rust, security, and credential-management reviewers.
Favor explicitness, low surprise, and comments that explain why a sharp edge exists.

## Project Snapshot

- Primary binary: `vault`
- Preferred server entrypoint: `vault serve`
- Deprecated standalone binary: `vaultd` (still present, but prints a deprecation note)
- Platform target today: macOS only for real secret storage and runtime injection
- Secret storage: macOS Keychain
- Metadata storage: SQLite at `.local/vault.db` unless `VAULT_DATABASE_URL` is set

This project exists to let agents use provider credentials without seeing or persisting the raw secrets in repo files or shell history.

## Build, Test, Run

Use these commands from the repo root:

```bash
cargo build
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all

cargo run -p vault-cli -- --help
cargo run -p vault-cli -- credential list
cargo run -p vault-cli -- profile list
cargo run -p vault-cli -- stats --json

vault serve
vault serve --background
vault serve status
vault serve stop
vault ui
```

Verification baseline before claiming code changes are done:

```bash
cargo test --workspace
```

## What Lives Where

Workspace crates:

- `crates/vault-cli` — main user-facing CLI (`credential`, `profile`, `run`, `stats`, `ui`, `serve`)
- `crates/vaultd` — Axum server library for dashboard, auth, SSE, stats, and proxy; standalone `vaultd` binary is deprecated
- `crates/vault-core` — shared domain types and `ProviderAdapter` trait
- `crates/vault-db` — SQLite store and repository methods
- `crates/vault-secrets` — macOS Keychain integration and trusted-app ACL handling
- `crates/vault-providers` — provider schemas and proxy/env adapters
- `crates/vault-policy` — lease issuance and environment policy checks
- `crates/vault-telemetry` — usage-event persistence and rollups

Important files:

- `README.md` — product-level overview and user-facing examples
- `CHANGELOG.md` — released and unreleased user-visible changes
- `docs/ROADMAP.md` — candidate future work, including audit follow-ups
- `docs/UAT.md` — release-gate UAT source of truth
- `docs/audits/2026-04-14-tob-baseline/SUMMARY.md` — consolidated security baseline
- `migrations/` — SQL schema and data migrations

## Canonical Runtime Flows

### 1. `vault run`

Main code: `crates/vault-cli/src/commands/run.rs`

Flow:

1. Load the named profile from SQLite
2. Load profile bindings
3. Skip `Proxy` bindings for env injection
4. Read secrets from macOS Keychain
5. Map them through the provider adapter into env vars
6. Issue a short-lived lease token and persist only its blake3 hash
7. Inject `VAULT_PROFILE`, `VAULT_AGENT`, `VAULT_LEASE_TOKEN`, and optional `VAULT_PROJECT`
8. Spawn the child process
9. Record a `vault` launch usage event for audit/stats

If Keychain ACL access fails, the CLI deliberately tries to surface a recovery-oriented error that tells the user to re-add the credential or allow the current binary in Keychain Access.

### 2. `vault serve` / `vault ui`

Main code:

- `crates/vault-cli/src/commands/serve.rs`
- `crates/vault-cli/src/commands/ui.rs`
- `crates/vaultd/src/lib.rs`
- `crates/vaultd/src/routes/mod.rs`

Behavior:

- Server binds to `127.0.0.1` only
- `vault serve --background` uses a PID file at `.local/vault.pid`
- `vault ui` auto-starts the server if needed, requests a PIN challenge, prints the PIN, and opens `/login?challenge=...`

### 3. Dashboard auth and live updates

Main code:

- `crates/vaultd/src/auth.rs`
- `crates/vaultd/src/routes/dashboard.rs`
- `crates/vaultd/src/routes/events.rs`

Auth model:

- `POST /api/auth/challenge` creates a 6-digit PIN challenge
- `POST /api/auth/login` exchanges challenge + PIN for a session cookie and CSRF token
- Session cookie is `HttpOnly` and `SameSite=Strict`
- CSRF checks are enforced for mutating dashboard routes
- SSE polling every 2 seconds is used for cross-process dashboard refresh

### 4. HTTP proxy

Main code: `crates/vaultd/src/routes/proxy.rs`

Flow:

1. Validate `x-vault-lease-token`
2. Resolve provider adapter
3. Find a `Proxy` or `Either` binding for the provider
4. Load the bound secret from Keychain
5. Forward the upstream request with provider-specific auth headers
6. Parse usage metadata when the adapter supports it
7. Persist a `UsageEvent`

## Provider Support

Schema-level providers currently include:

- `openai`
- `anthropic`
- `openrouter`
- `twitterapi`
- `github`
- `tavily`
- `coingecko`

Full runtime adapters in `crates/vault-providers/src/registry.rs` currently exist only for:

- `openai`
- `anthropic`
- `twitterapi`

Be careful not to assume every schema-backed provider has full proxy usage parsing.

## Data and Security Boundaries

- Secrets do not belong in SQLite, repo files, `.env`, or logs
- Keychain item service name is `ai.zyr1.vault`
- Leases persist only hashed tokens, never raw tokens
- Monetary storage is integer microdollars in the DB; JSON/UI still expose dollar values
- Dashboard and proxy are loopback-only by design
- `prod` credentials are blocked by default via `PolicyService`

When changing secret-handling code, audit for:

- raw `String` copies of secrets
- new buffers that outlive their need
- accidental logs of identifiers or secret material
- places where provider support is assumed instead of checked

## Current Conventions

- Rust edition 2024, stable toolchain
- IDs are UUID strings
- Timestamps are ISO 8601 strings in SQLite
- `vault-db` uses manual row mapping, not `FromRow` derive
- Askama templates are rendered manually; do not rely on `IntoResponse` from `askama_axum`
- `vault-cli` computes the default DB path from the workspace root, not from the current shell cwd

## Known Gotchas

- `vaultd` standalone binary exists but is deprecated; use `vault serve`
- macOS-only codepaths are intentional; many secret/proxy operations bail on non-macOS
- `vault run` only injects bindings whose mode is not `Proxy`
- `vaultd` CORS is intentionally narrow and only allows loopback origin
- Background-server health checks and PID-file cleanup matter; do not break them casually
- UAT is a real release gate here; if you change user-facing behavior, check `docs/UAT.md`

## GitNexus — Code Intelligence

This project is indexed by GitNexus as **credential-broker**.

### Always Do

- **MUST run impact analysis before editing any function, class, or method.** Use `gitnexus_impact({target: "symbolName", direction: "upstream", repo: "credential-broker"})`.
- **MUST run `gitnexus_detect_changes()` before committing** to verify scope.
- **MUST warn the user** before proceeding if impact analysis is HIGH or CRITICAL.
- For docs-only edits, symbol-level impact analysis is not required.

### Recommended Flow

1. Read `gitnexus://repo/credential-broker/context`
2. Use `gitnexus_query(...)` or `gitnexus_context(...)` for code understanding
3. Read process resources when available
4. Fall back to source reading if GitNexus artifacts are incomplete

### Important Local Caveat

In this repo, GitNexus metadata can report `Status: up-to-date` while deeper query/process resources fail with:

```text
LadybugDB not found at .gitnexus/lbug
```

If that happens:

1. Do not trust process/query coverage blindly
2. Fall back to direct source reading for the current task
3. If you need GitNexus fully restored, use the CLI repair path:

```bash
/opt/homebrew/bin/gitnexus clean
/opt/homebrew/bin/gitnexus analyze
```

### Useful Commands

```bash
/opt/homebrew/bin/gitnexus status
/opt/homebrew/bin/gitnexus analyze
/opt/homebrew/bin/gitnexus wiki
```

### Self-Check Before Finishing

Before completing code work:

1. Impact analysis run for each modified symbol
2. No HIGH/CRITICAL warnings ignored
3. `gitnexus_detect_changes()` checked before commit
4. `cargo test --workspace` run and read

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **credential-broker** (2349 symbols, 3923 relationships, 197 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

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
