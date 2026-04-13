# credential-broker

A local credential broker that lets coding agents and scripts access API keys without seeing the raw secrets. Store credentials once in macOS Keychain, create named profiles that bundle providers together, then launch agent subprocesses or proxy HTTP requests — all with short-lived leases and full usage tracking.

## Why

Coding agents (Codex, Claude Code, Cursor, etc.) need API keys to call LLMs, search APIs, and other services. Pasting keys into `.env` files or shell history is insecure and hard to audit. credential-broker solves this by:

- **Storing secrets in macOS Keychain** with trusted-application ACLs — not in files
- **Injecting credentials at runtime** via environment variables or an authenticated HTTP proxy
- **Issuing short-lived leases** so access is time-bounded and auditable
- **Tracking every request** with provider, model, token count, and cost estimates
- **Web dashboard** for real-time monitoring with PIN-based auth and live updates

## Installation

**One-liner** (macOS only):
```bash
curl -fsSL https://raw.githubusercontent.com/zengyuzhi/credential-broker/main/install.sh | bash
```

**From source** (requires Rust toolchain):
```bash
cargo install --git https://github.com/zengyuzhi/credential-broker vault-cli
```

**Manual download**: grab the binary from [GitHub Releases](https://github.com/zengyuzhi/credential-broker/releases) for your architecture (Apple Silicon or Intel).

> **Note**: This project's binary is named `vault`. It is unrelated to HashiCorp Vault — this is a local-only credential broker for coding agents.

## Quick Start

```bash
# Build (produces 'vault' binary in target/debug/)
cargo build -p vault-cli

# Store an API key (prompts securely, never shown in shell history)
vault credential add openai work-main --kind api_key --env work

# Create a profile and bind the credential
vault profile create coding
vault profile bind coding openai <credential-id> --mode inject

# Launch an agent with injected credentials
vault run --profile coding --agent codex -- your-command-here

# Open the web dashboard (auto-starts the server)
vault ui

# Check usage stats
vault stats
vault stats --json              # machine-readable output
vault stats --provider openai   # filter by provider
```

## Features

### Credential Management

```bash
vault credential add <provider> <label> [--kind api_key] [--env work]
vault credential list
vault credential enable <id>
vault credential disable <id>
vault credential remove <id> --yes
```

Secrets are stored in macOS Keychain under service `ai.zyr1.vault` with trusted-application ACLs. The CLI binary is pre-authorized during credential creation so `vault run` works without Keychain prompts.

### Profiles

Profiles bundle multiple provider credentials into a named configuration:

```bash
vault profile create <name>
vault profile list
vault profile show <name>
vault profile bind <profile> <provider> <credential-id> --mode inject|proxy|either
```

### Environment Injection (`vault run`)

Launch any command with provider credentials injected as environment variables:

```bash
vault run --profile coding --agent codex --project my-app -- python main.py
```

This:
1. Resolves all `inject`/`either` bindings for the profile
2. Reads secrets from Keychain (non-interactive, no prompts)
3. Maps them to provider-specific env vars (e.g., `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`)
4. Issues a short-lived lease (blake3-hashed token)
5. Injects `VAULT_PROFILE`, `VAULT_AGENT`, `VAULT_LEASE_TOKEN`, `VAULT_PROJECT`
6. Spawns the child process
7. Records a launch event for auditing

### HTTP Proxy

For providers bound with `proxy` or `either` mode, the server forwards requests with credentials injected server-side:

```bash
# Start the server
vault serve

# Agent sends requests to the proxy instead of directly to the provider
curl -X POST http://127.0.0.1:8765/v1/proxy/openai/v1/chat/completions \
  -H "x-vault-lease-token: <token>" \
  -H "content-type: application/json" \
  -d '{"model": "gpt-4", "messages": [{"role": "user", "content": "hello"}]}'
```

The proxy:
- Authenticates via lease token (blake3 hash lookup + expiry check)
- Injects the real API key (agent never sees it)
- Forwards to the upstream provider
- Parses the response for usage data (tokens, model, cost)
- Records a telemetry event

### Web Dashboard

A browser-based dashboard for monitoring credentials, sessions, and usage — no npm or build step required.

```bash
vault ui    # auto-starts server, generates PIN, opens browser
```

**Pages:** Home (overview stats), Credentials (enable/disable toggle), Profiles (expandable bindings), Stats (provider filter), Sessions (active/expired leases)

**Security:**
- PIN-based auth (6-digit, 5-minute expiry, burned after 5 failed attempts)
- Per-session CSRF tokens on all mutating requests
- httpOnly + SameSite=Strict cookies
- CORS locked to `127.0.0.1:8765`
- Secrets never appear in any dashboard response

**Live updates:** SSE endpoint polls SQLite every 2 seconds — changes from CLI commands (e.g., `vault run`, `vault credential disable`) appear in the dashboard automatically.

### Server Management

```bash
vault serve                  # foreground (blocks until Ctrl+C)
vault serve --background     # background with PID file
vault serve --port 9000      # custom port
vault serve status           # check if running
vault serve stop             # stop background server
```

`vault ui` auto-starts the server in the background if it's not already running.

### Usage Stats

```bash
vault stats                          # text output
vault stats --json                   # JSON array for scripting
vault stats --provider openai        # filter by provider
vault stats --json --provider openai # combined
```

Shows aggregated usage per provider: request count, prompt/completion tokens, estimated cost, last used timestamp. Also available via HTTP at `GET /v1/stats/providers`.

## Supported Providers

| Provider | Inject | Proxy | Required Field |
|----------|--------|-------|----------------|
| OpenAI | Yes | Yes | `api_key` |
| Anthropic | Yes | Yes | `api_key` |
| OpenRouter | Yes | Yes | `api_key` |
| TwitterAPI | Yes | Yes | `api_key` |
| GitHub | Yes | Yes | `token` |
| Tavily | Yes | Yes | `api_key` |
| CoinGecko | Yes | Yes | `api_key` |

Full proxy adapters (with usage parsing) exist for OpenAI, Anthropic, and TwitterAPI. Other providers support inject mode and basic proxy forwarding.

## Architecture

Single CLI binary with embedded HTTP server, six library crates:

```
vault-cli (binary, includes vault serve)
    |
    +-- vaultd (library: HTTP server, dashboard, proxy)
    +-- vault-policy
    +-- vault-telemetry
    +-- vault-providers
    +-- vault-secrets
    +-- vault-db
    +-- vault-core
```

| Crate | Responsibility |
|-------|---------------|
| `vault-core` | Domain types, `ProviderAdapter` trait, `VaultError` |
| `vault-db` | SQLite persistence via sqlx (credentials, profiles, bindings, leases, usage events, UI sessions) |
| `vault-secrets` | `SecretStore` trait + macOS Keychain implementation with trusted-app ACLs |
| `vault-providers` | Provider adapters (env mapping, upstream URLs, usage parsing) |
| `vault-policy` | Lease issuance (UUID + blake3 hash) and environment policy enforcement |
| `vault-telemetry` | Usage event recording and rollup queries |
| `vaultd` | Axum HTTP server: dashboard pages, auth, SSE, proxy routes (now a library crate) |

## Data Storage

- **Secrets**: macOS Keychain (never in files or database)
- **Metadata**: SQLite at `.local/vault.db` (credentials, profiles, bindings, leases, usage events, UI sessions)
- **PID file**: `.local/vault.pid` (background server process tracking)
- **Override**: Set `VAULT_DATABASE_URL` to use a different SQLite path

Migrations are auto-applied on first connection.

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `VAULT_DATABASE_URL` | Override SQLite path (default: `.local/vault.db`) |
| `VAULT_DEBUG_RUN` | Set to `1` for debug logging in `vault run` |
| `VAULT_TRUSTED_APP_PATHS` | Colon-separated extra executable paths for Keychain ACL recovery |

## Development

```bash
cargo build                          # build all crates
cargo test                           # run all tests (73 tests)
cargo clippy --workspace --all-targets -- -D warnings  # lint
cargo fmt --all                      # format
cargo run -p vault-cli -- <subcmd>   # run CLI
vault serve                          # start server on 127.0.0.1:8765
```

Requires:
- Rust stable toolchain (edition 2024)
- macOS (for Keychain integration)

## Security Model

- Secrets are stored in macOS Keychain with trusted-application ACLs, not in files
- Keychain reads are non-interactive — unauthorized access fails fast with actionable error messages
- The `security` CLI is invoked by absolute path (`/usr/bin/security`) to prevent PATH hijacking
- Secrets are piped via stdin (not CLI arguments) to avoid process-list exposure
- Leases are time-bounded (default 60 minutes) with blake3-hashed tokens
- Production credentials are blocked by default unless `allow_prod` is explicitly set
- Dashboard uses PIN-based auth with per-session CSRF tokens and strict CORS

## License

MIT
