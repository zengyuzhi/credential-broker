# Changelog

All notable changes to credential-broker will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

User-visible bullets live here; implementation detail lives in `git log`.

## [Unreleased]

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.1.0] - 2026-04-14

Initial personal release. macOS-only. Unsigned.

### Added

- **Credential management**: store, list, enable, disable, and remove credentials backed by the macOS Keychain under service `ai.zyr1.vault`. Trusted-application ACLs pre-authorize the `vault` binary so later reads are non-interactive.
  - `vault credential add <provider> <label> [--kind api_key] [--env work|prod]`
  - `vault credential list`
  - `vault credential enable <id>` / `disable <id>` / `remove <id> --yes`
- **Profiles**: bundle multiple provider credentials into a named configuration with per-provider access modes.
  - `vault profile create <name>`
  - `vault profile list`
  - `vault profile show <name>`
  - `vault profile bind <profile> <provider> <credential-id> --mode inject|proxy|either`
- **`vault run`**: launch any command with provider credentials injected as environment variables. Issues a short-lived blake3-hashed lease token, records a launch event, and spawns the child process with `VAULT_PROFILE`, `VAULT_AGENT`, `VAULT_LEASE_TOKEN`, `VAULT_PROJECT` set.
- **HTTP proxy**: `POST /v1/proxy/{provider}/{*path}` authenticates via lease token and forwards to the upstream provider with the real API key injected server-side. Usage (tokens, model, cost estimate) parsed and recorded for OpenAI, Anthropic, and TwitterAPI.
- **Supported providers**: OpenAI, Anthropic, OpenRouter, TwitterAPI, GitHub, Tavily, CoinGecko (inject mode for all; full proxy adapters with usage parsing for the first three).
- **Web dashboard**: browser UI for monitoring credentials, profiles, sessions, and usage. No npm/build step — askama templates + Pico CSS + htmx via CDN.
  - Pages: Home (overview stats), Credentials (enable/disable toggle), Profiles (expandable bindings), Stats (provider filter), Sessions (active/expired leases).
  - PIN-based auth: 6-digit, 5-minute expiry, burned after 5 failed attempts, 3/min rate limit.
  - Per-session CSRF tokens on all mutating requests; httpOnly + `SameSite=Strict` cookies; CORS locked to `127.0.0.1:8765`.
  - Live updates: SSE endpoint polls SQLite every 2 seconds so CLI-side changes appear in the dashboard automatically.
  - Mobile-responsive layout with hamburger menu; respects `prefers-reduced-motion`.
- **`vault serve`**: embedded HTTP server (formerly standalone `vaultd` binary).
  - `vault serve` (foreground)
  - `vault serve --background` (PID file at `.local/vault.pid`, detaches via `process_group(0)`)
  - `vault serve --port <n>`
  - `vault serve status` / `vault serve stop`
- **`vault ui`**: auto-starts the server in the background if not already running, generates a PIN, and opens the dashboard in the default browser.
- **`vault stats`**: usage rollups aggregated per provider (request count, prompt/completion tokens, estimated cost, last used timestamp).
  - `vault stats` — text output
  - `vault stats --json` — machine-readable array for scripting
  - `vault stats --provider <name>` — filter to one provider
  - Also exposed via `GET /v1/stats/providers`
- **GitHub Actions CI**: fmt check, clippy with `-D warnings`, and `cargo test --workspace` on every PR and push to `main`.
- **GitHub Actions release workflow**: triggers on `v*` tags, builds `aarch64-apple-darwin` and `x86_64-apple-darwin` matrix, packages `vault-<target>.tar.gz`, and publishes a GitHub Release with auto-generated notes.
- **`install.sh`**: one-command installer (`curl | bash`). macOS-only gate, architecture detection, GitHub API fetch of latest release, download + extract to `~/.local/bin/`, PATH guidance, upgrade support.
- **Installation documentation**: README covers three install paths (curl-pipe, `cargo install --git`, manual download) plus a note disambiguating from HashiCorp Vault.

### Security

- Secrets never leave macOS Keychain — not stored in files or SQLite.
- The `security` CLI is invoked by absolute path (`/usr/bin/security`) to prevent PATH hijacking.
- Secrets are piped via stdin (`-w` as last arg) to `security add-generic-password` rather than passed as CLI args to avoid process-list exposure.
- Leases are time-bounded (default 60 minutes) with blake3-hashed tokens; raw tokens never persisted.
- Production-env credentials (`--env prod`) are blocked from injection unless `allow_prod` is set.
- Dashboard PIN replay prevented — a used challenge returns `409 Conflict`.

### Known limitations

- **macOS only.** Keychain integration is `#[cfg(target_os = "macos")]`; no Linux/Windows support yet.
- **Unsigned binary.** On macOS 15+ Gatekeeper may quarantine the downloaded binary. Workaround: `xattr -d com.apple.quarantine ~/.local/bin/vault` after install. Proper signing + notarization tracked in the roadmap.
- **Personal-scale polish.** The product is shipped as-is for the author's own use. Expect rough edges; file issues, but no SLA.
- **Full usage parsing only for three providers.** OpenAI, Anthropic, and TwitterAPI adapters parse responses for token counts and cost estimates; other providers support inject + basic proxy forwarding only.
- **No automatic CHANGELOG generation.** Entries are written by hand.

[Unreleased]: https://github.com/zengyuzhi/credential-broker/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/zengyuzhi/credential-broker/releases/tag/v0.1.0
