# Changelog

All notable changes to credential-broker will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

User-visible bullets live here; implementation detail lives in `git log`.

## [Unreleased]

### Added

- **`vault upgrade` self-update command** for macOS installs managed directly from GitHub Releases.
  - `vault upgrade --check` reports whether a newer release is available without downloading artifacts.
  - `vault upgrade --dry-run` performs release lookup, minisign verification, checksum validation, and extraction without replacing the installed binary.
  - `vault upgrade` refuses to run while a background `vault serve` daemon is active and blocks downgrades unless you explicitly opt in with `--to <version> --force`.

### Changed

- **Phase 0 guidance now labels today's launcher-style flows as the compatibility baseline.** The architecture docs, phase plan, and CLI help distinguish env injection from the preferred brokered-access model, and no migration is required yet.
- **Keychain service namespace renamed to `dev.credential-broker.vault`.** New credentials use the generic product-scoped name, while older private-namespace secret refs are migrated forward for backward compatibility.

### Deprecated

### Removed

### Fixed

- `vault --version` / `vault -V` now report the version. v0.1.0 documentation advertised the flag but clap wasn't wired up for it (`#[command(version)]` missing). Install-script end-to-end smoke test caught the gap.
- Installed release binaries now fall back to `~/.local/share/credential-broker/vault.db` when the build-time workspace path is unavailable, and `vault upgrade --check` / `vault serve status` no longer create the state directory just to read PID state. This fixes the post-self-update `Permission denied` panic from v0.1.1.

### Security

- **Release artifacts now require minisign provenance.** The release workflow creates a draft release with tarballs plus `SHA256SUMS`; the maintainer signs `SHA256SUMS` locally with the checked-in public key embedded into `vault`, uploads `SHA256SUMS.minisig`, and only then publishes the release.

- **Secret-memory wipe via `zeroize`** — API keys, lease tokens, and session tokens are now held in `Zeroizing<String>` at their primary allocation sites so the heap bytes are overwritten on drop rather than lingering until the allocator reuses the pages. `ResolvedCredential.fields` wipes on drop via a custom `Drop` impl. Covers audit findings ZA-0001..0007 + SE-05. Added workspace dep `zeroize = "1"`.
- **Lease `ttl_minutes` validated at the type boundary** — `issue_lease` now accepts `NonZeroU32` instead of `i64`, rejecting zero, negative, and oversized values at construction time rather than silently producing immediately-expired or panicking leases. (Audit SE-07.)
- **Session-expiry timestamp overflow no longer collapses to `now`** — `checked_add_signed(...).unwrap_or(now)` in the PIN challenge and UI session refresh paths is replaced with propagated errors, so overflowed arithmetic surfaces as a 500 instead of creating an immediately-expired session. (Audit SE-06.)
- **Latency telemetry saturates instead of truncating** — `u128` microsecond elapsed time is now converted via `i64::try_from(...).unwrap_or(i64::MAX)` so overflow produces a sticky max value rather than a wrap to a small number. (Audit SE-08.)
- **Monetary accumulation moves to integer microdollars** — `UsageEvent.estimated_cost_usd: Option<f64>` → `estimated_cost_micros: Option<i64>`. SQL SUM now accumulates exactly; display code converts to dollars once at render time. Migration `0003_usage_events_cost_micros.sql` backfills existing data. (Audit SE-09.)
- **Constant-time PIN hash comparison** — dashboard PIN verification previously compared `String` values, leaking timing information that could defeat the 5-attempt burn. Now uses `subtle::ConstantTimeEq`. Added workspace dep `subtle = "2"`. (Audit finding SE-01 CRITICAL, Trail of Bits sharp-edges 2026-04-14.)
- **Constant-time CSRF token comparison** — same timing leak applied to the session CSRF token; empty-token guard ran after the compare. Reordered and switched to `ConstantTimeEq`. (SE-02 HIGH.)
- **Rate-limit key no longer spoofable** — the PIN challenge rate limiter keyed on `x-forwarded-for` with `host` fallback, both client-controlled. A local process rotating headers could get unlimited PIN attempts. Since vaultd binds to `127.0.0.1` only, rate limiting now uses a fixed server-side key. (SE-03 HIGH.)
- **Removed dangerous default `SecretStore::put`** — the trait exposed an `put` method that stored secrets with no Keychain ACL; any app on the system could read them. No production code called it, but its presence invited future misuse. The sole supported write path is now `MacOsKeychainStore::put_with_access` (trusted-app ACL required). (SE-04 HIGH.)
- Full baseline report: `docs/audits/2026-04-14-tob-baseline/SUMMARY.md`. 16 additional findings triaged to `docs/ROADMAP.md`; 6 accepted.

### Quality

- **User-acceptance-test release gate formalized** at [`docs/UAT.md`](docs/UAT.md) with 23 entries across 6 capability areas (CLI, dashboard, serve lifecycle, proxy, install/migration, security regression). Release procedure step 5 now runs the UAT pass before CHANGELOG rotation; golden-path failure or threshold miss (4/4 golden + ≥95% AUTO:ANY + ≥80% MANUAL:USER) blocks tag push. First AI-produced baseline run-log at [`docs/uat-runs/2026-04-14-v0.1.1-pre-ai.md`](docs/uat-runs/2026-04-14-v0.1.1-pre-ai.md) (`status: baseline-only`); v0.1.1 still requires a human-run UAT before tag. Canonical capability: `uat-release-gate`.

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
