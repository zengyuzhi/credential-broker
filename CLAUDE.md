# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```bash
cargo build                          # build all crates
cargo test                           # run all tests
cargo clippy --workspace --all-targets -- -D warnings  # lint (strict)
cargo run -p vault-cli -- <subcmd>   # run CLI (e.g. credential list)
cargo run -p vaultd                  # start daemon on 127.0.0.1:8765
cargo test -p vault-db               # test a single crate
```

The SQLite database lives at `.local/vault.db` (gitignored). Override with `VAULT_DATABASE_URL` env var. Migrations in `migrations/` are auto-applied by `vault-db` on pool creation via `sqlx::migrate!`.

## Architecture

This is a local credential broker that lets coding agents and scripts access API keys without seeing the raw secrets. Two binaries, six library crates:

```
vault-cli (binary)          vaultd (binary)
    │                           │
    ├── vault-policy            ├── (axum routes - stub)
    ├── vault-telemetry         │
    ├── vault-providers         │
    ├── vault-secrets           │
    ├── vault-db                │
    └── vault-core ─────────────┘
```

### Crate responsibilities

- **vault-core** — Domain types (`Credential`, `Profile`, `Lease`, `UsageEvent`), the `ProviderAdapter` async trait, and `VaultError`. No internal deps — everything depends on this.
- **vault-db** — `Store` wraps `SqlitePool`. Sub-modules (`credentials`, `profiles`, `bindings`, `leases`) each implement CRUD. All queries use sqlx macros.
- **vault-secrets** — `SecretStore` trait with a macOS Keychain implementation (`security-framework`). Secrets stored under service `ai.zyr1.vault` with ref format `<service>:<credential_id>:<field_name>`.
- **vault-providers** — `ProviderAdapter` implementations (OpenAI, Anthropic, TwitterAPI). `registry::adapter_for()` returns the right adapter. `schema.rs` has static `ProviderSchema` definitions for 7 providers (only 3 have full adapters).
- **vault-policy** — Lease issuance (`issue_lease` generates UUID token, blake3-hashes it) and `PolicyService` (blocks prod credentials unless `allow_prod` is set).
- **vault-telemetry** — `TelemetryWriter` and `StatsSummary` (both stubs currently).
- **vault-cli** — Clap-derived CLI. Fully working: `credential add/list/enable/disable/remove`, `run --profile <name> -- <cmd>`. Stubs: `profile`, `stats`.
- **vaultd** — Axum HTTP daemon. Only `GET /health` and `GET /stats/providers` (empty stub) are wired.

### Key data flow: `vault run`

1. CLI resolves profile name → loads profile bindings from DB
2. For each binding, reads secret from macOS Keychain via `SecretStore`
3. Provider adapter's `env_map()` builds env vars (e.g. `OPENAI_API_KEY`)
4. `issue_lease()` generates a session token (raw + blake3 hash stored in DB)
5. Injects `VAULT_PROFILE`, `VAULT_AGENT`, `VAULT_LEASE_TOKEN`, `VAULT_PROJECT` into env
6. Spawns child process with the combined environment

### Access modes

Credentials bind to profiles with a `mode`: `Inject` (env vars), `Proxy` (vaultd forwards requests), or `Either`. Only Inject is implemented.

## Conventions

- Rust edition 2024, stable toolchain
- All IDs are UUID v4 strings (not native UUID columns — SQLite stores them as TEXT)
- Timestamps are ISO 8601 strings in SQLite (`chrono` for serialization)
- Errors: `VaultError` (thiserror) for domain errors, `anyhow::Result` for plumbing
- macOS-only for now: `vault-secrets` uses `security-framework` behind `#[cfg(target_os = "macos")]`

## Gotchas

- `security-framework` v3 does NOT bind Keychain ACL APIs (`SecAccessCreate`, `SecTrustedApplicationCreateFromPath`). Trusted-app ACLs use `/usr/bin/security` CLI — always absolute path, never bare `security`.
- macOS `security add-generic-password`: place `-w` as the **last** argument with no value to read the password from stdin (avoids process-list exposure).
- `MacOsKeychainStore` is a unit struct — construct with `MacOsKeychainStore`, not `MacOsKeychainStore::default()` (clippy `default_constructed_unit_structs`).
- Tests use `std::sync::Mutex` for DB URL serialization across async tests — annotate with `#[allow(clippy::await_holding_lock)]`.
- Implementation plans live in `docs/plans/` — check there before starting new work.
