# credential-broker

A local credential broker that lets coding agents and scripts access API keys without seeing the raw secrets. Store credentials once in macOS Keychain, create named profiles that bundle providers together, then launch agent subprocesses or proxy HTTP requests — all with short-lived leases and full usage tracking.

## Why

Coding agents (Codex, Claude Code, Cursor, etc.) need API keys to call LLMs, search APIs, and other services. Pasting keys into `.env` files or shell history is insecure and hard to audit. credential-broker solves this by:

- **Storing secrets in macOS Keychain** with trusted-application ACLs — not in files
- **Injecting credentials at runtime** via environment variables or an authenticated HTTP proxy
- **Issuing short-lived leases** so access is time-bounded and auditable
- **Tracking every request** with provider, model, token count, and cost estimates

## Quick Start

```bash
# Build
cargo build

# Store an API key (prompts securely, never shown in shell history)
cargo run -p vault-cli -- credential add openai work-main --kind api_key --env work

# Create a profile and bind the credential
cargo run -p vault-cli -- profile create coding
cargo run -p vault-cli -- profile bind coding openai <credential-id> --mode inject

# Launch an agent with injected credentials
cargo run -p vault-cli -- run --profile coding --agent codex -- your-command-here

# Check usage stats
cargo run -p vault-cli -- stats
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

### HTTP Proxy (`vaultd`)

For providers bound with `proxy` or `either` mode, the daemon forwards requests with credentials injected server-side:

```bash
# Start the daemon
cargo run -p vaultd

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

### Usage Stats

```bash
vault stats [--provider openai]
```

Shows aggregated usage per provider: request count, prompt/completion tokens, estimated cost, last used timestamp. Stats are also available via HTTP at `GET /stats/providers`.

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

Two binaries, six library crates:

```
vault-cli (binary)          vaultd (binary)
    |                           |
    +-- vault-policy            +-- vault-policy
    +-- vault-telemetry         |
    +-- vault-providers         +-- vault-providers
    +-- vault-secrets           +-- vault-secrets
    +-- vault-db                +-- vault-db
    +-- vault-core -------------+
```

| Crate | Responsibility |
|-------|---------------|
| `vault-core` | Domain types, `ProviderAdapter` trait, `VaultError` |
| `vault-db` | SQLite persistence via sqlx (credentials, profiles, bindings, leases, usage events) |
| `vault-secrets` | `SecretStore` trait + macOS Keychain implementation with trusted-app ACLs |
| `vault-providers` | Provider adapters (env mapping, upstream URLs, usage parsing) |
| `vault-policy` | Lease issuance (UUID + blake3 hash) and environment policy enforcement |
| `vault-telemetry` | Usage event recording and rollup queries |

## Data Storage

- **Secrets**: macOS Keychain (never in files or database)
- **Metadata**: SQLite at `.local/vault.db` (credentials, profiles, bindings, leases, usage events)
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
cargo test                           # run all tests (49 tests)
cargo clippy --workspace --all-targets -- -D warnings  # lint
cargo fmt --all                      # format
cargo run -p vault-cli -- <subcmd>   # run CLI
cargo run -p vaultd                  # start daemon on 127.0.0.1:8765
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

## License

MIT
