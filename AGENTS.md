# Repository Guidelines

## Project Structure & Module Organization
`credential-broker` is a Rust workspace. Core code lives in `crates/`: `vault-cli` is the user-facing binary, `vaultd` serves the local dashboard/proxy, and the remaining crates handle domain types, SQLite persistence, Keychain access, provider adapters, policy, and telemetry. Integration tests live under `crates/*/tests`. Supporting material sits in `docs/`, schema changes in `migrations/`, runnable examples in `examples/`, and release/spec work in `openspec/`.

## Build, Test, and Development Commands
- `cargo build -p vault-cli`: build the main `vault` binary.
- `cargo test --workspace`: run the full test suite used by CI.
- `cargo clippy --workspace --all-targets -- -D warnings`: enforce lint cleanliness.
- `cargo fmt --all`: format the workspace.
- `cargo run -p vault-cli -- --help`: inspect CLI entry points locally.
- `cargo run -p vault-cli -- serve`: start the loopback-only server for dashboard/proxy work.

## Coding Style & Naming Conventions
Use stable Rust with `rustfmt` defaults (4-space indentation, trailing commas where formatter adds them). Keep modules focused and prefer explicit names such as `issue_lease`, `store_smoke`, or `serve_state_path`. Follow existing crate naming (`vault-*`) and keep SQL migrations ordered numerically (`0001_init.sql`). Avoid logging secrets, raw tokens, or Keychain material.

## Testing Guidelines
Add unit tests next to the code they cover when practical, and place cross-crate or CLI flows in `crates/<crate>/tests/*.rs`. Favor scenario-driven integration test names like `upgrade_local_fixtures.rs`. Before opening a PR, run `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo fmt --all -- --check`.

## Commit & Pull Request Guidelines
Recent history uses Conventional Commit prefixes: `feat(...)`, `fix(...)`, `docs(...)`, `chore(...)`, `style(...)`. Keep subjects imperative and scoped, for example `fix(vault-cli): use user-owned state dir`. PRs should summarize behavior changes, list verification commands, and call out any updates to `CHANGELOG.md`, `docs/UAT.md`, or `docs/ARCHITECTURE.md`. Include screenshots only when dashboard/UI output changes.

## Security & Configuration Tips
This project is macOS-first and intentionally keeps secrets out of files. Never commit `.env` secrets, sample API keys, or copied Keychain values. Prefer test fixtures and synthetic release assets over real credentials when working on upgrade, proxy, or telemetry flows.
