# credential-broker

A local capability broker for coding agents and scripts. Store credentials once in macOS Keychain, then use the strongest access path a workflow supports: brokered HTTP access when a tool can talk to the local vault, or user-operated env-injection compatibility workflows when a legacy command cannot yet integrate cleanly.

## Why

Coding agents (Codex, Claude Code, Cursor, etc.) need API keys to call LLMs, search APIs, and other services. Pasting keys into `.env` files or shell history is insecure and hard to audit. credential-broker solves this by:

- **Storing secrets in macOS Keychain** with trusted-application ACLs — not in files
- **Brokering HTTP requests when supported** so agents can use APIs without ever receiving raw keys on the supported path
- **Keeping env injection available only as a user-operated compatibility path** for legacy tools that still need child-process credentials
- **Issuing short-lived leases** so access is time-bounded and auditable
- **Tracking every request** with provider, model, token count, and cost estimates
- **Web dashboard** for real-time monitoring with PIN-based auth and live updates

Agent-readable files such as `.env`, JSON, YAML, copied config snippets, and prompt transcripts are not part of the trust boundary. When plaintext migration is supported, it should be a user-triggered one-time import, not an agent path.

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

> **macOS Gatekeeper**: the released binary is unsigned. If macOS refuses to run it with a "cannot be verified" dialog, strip the quarantine attribute once:
> ```bash
> xattr -d com.apple.quarantine ~/.local/bin/vault
> ```
> Code signing + notarization is on the [roadmap](./docs/ROADMAP.md).

Release history lives in [CHANGELOG.md](./CHANGELOG.md).
The long-term product direction and design principles live in [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md).

Phase 1 status: the broker core domain model (connectors, capabilities, grants, bundles, sessions) is now available alongside the existing compatibility baseline. Today's `credential`, `profile`, `run`, `serve`, `ui`, and `upgrade` flows continue to work unchanged. The new broker commands (`connector`, `capability`, `grant`, `bundle`, `session`) introduce the target capability model. Use `bundle from-profile` to bridge existing profiles into the new model. Brokered access is the target; env injection remains available only as a user-operated compatibility path for legacy tools.

## Current Implementation Reality

The target architecture is broker-first, but the current runtime surfaces are still transitional. The most important constraints to understand today are:

- There is **no vault MCP server yet**. MCP-only agents still need an adapter layer or another host surface to talk to `vault`.
- The HTTP proxy at `/v1/proxy/{provider}/{*path}` authenticates broker-native traffic with **`Authorization: Bearer <session-token>`** issued by `vault session issue`. The legacy **`x-vault-lease-token`** header remains as a compatibility path for `vault run` child processes. The wire contract is documented in [docs/runtime-contract.md](./docs/runtime-contract.md).
- `vault run` is still the compatibility path for tools that only accept environment variables, and the child process can read the injected credentials directly.
- Lease tokens are **short-lived but reusable until expiry**, not one-time-use codes, so they must be treated as sensitive runtime credentials.
- Any step that requires entering a raw secret must still be completed by the **user**, not by the agent.

## Upgrading

`vault` can self-update from GitHub Releases on macOS:

```bash
vault upgrade --check
vault upgrade --dry-run
vault upgrade
```

- `vault upgrade --check` only compares your installed version with the latest release.
- `vault upgrade --dry-run` runs the release lookup, signature verification, checksum verification, and extraction steps without replacing the current binary.
- `vault upgrade` refuses to run while a background `vault serve` daemon is active; stop it first with `vault serve stop`, then restart it after the upgrade.
- Downgrades and same-version reinstalls are blocked by default. Use `vault upgrade --to <version> --force` only when you explicitly want that rollback path.

## Typical Workflows

### 1. Legacy tool compatibility flow

Use this when the tool only knows how to read environment variables from its child process.

```bash
# Build locally (or install from the release page)
cargo build -p vault-cli

# Store the credential once in macOS Keychain
vault credential add openai work-main --kind api_key --env work

# Find the UUID you need for later commands
vault credential list

# Create a profile and bind the credential to it
vault profile create coding
vault profile bind coding openai <credential-id> --mode inject

# Launch the legacy command with injected env vars
vault run --profile coding --agent legacy-tool -- your-command-here

# Inspect what happened afterwards
vault stats --provider openai
vault ui
```

This is today's quickest path, but it is a compatibility path only. The child process can read the injected secret, so it is not the supported agent-security model.

### 2. Broker-core flow (Phase 1 primitives)

Use this when you want to model access as connector -> capability -> grant -> bundle -> session instead of handing a tool a raw key.

```bash
# Store the credential and find its UUID
vault credential add openai work-main --kind api_key --env work
vault credential list

# Register a connector backed by that credential
vault connector add work-openai --provider openai --credential <credential-id>

# Define one named capability label on that connector
vault capability add openai.responses.create --connector work-openai \
  --description "Create OpenAI responses through the broker"

# Find the capability UUID, then grant an agent access to it
vault capability list --connector work-openai
vault grant add --agent codex --capability <capability-id> --ttl 60

# Group grants into a bundle and issue a scoped session token
vault bundle create coding-bundle --description "Main coding workflow"
vault bundle add-grant coding-bundle <grant-id>
vault session issue --bundle coding-bundle --agent codex --project credential-broker --ttl 30

# Start the local broker surfaces
vault serve --background
vault ui
```

`openai.responses.create` is currently a user-defined capability name, not an auto-discovered upstream endpoint. Choose a stable action label that will still make sense in grants, bundles, and audit logs. Today `vault capability add` does not require you to know the raw HTTP path.

This is the target security direction, but the runtime path is still transitional. Phase 1.1 is where session-backed broker access becomes a real agent integration surface. Phase 2 is where brokered model access becomes the default day-to-day path.

### 3. Bridge an existing profile into the broker model

Use this when you already have compatibility profiles and want to move toward bundles and grants without rebuilding everything by hand.

```bash
# Review the old profile
vault profile show coding

# Convert it into a bundle
vault bundle from-profile coding --yes

# Inspect the resulting bundle and active grants
vault bundle show coding
vault grant list
```

The conversion creates broad wildcard grants for the profile's existing bindings, so `--yes` is required to make that trade-off explicit.

## CLI Reference

### Root command

- `vault --help`
  Prints the top-level command list and short descriptions.
- `vault --version`
  Prints the installed version. Useful before `vault upgrade` or when debugging a release issue.

### `vault credential`

Use `vault credential` to manage stored secret material. Credentials are kept in macOS Keychain under service `dev.credential-broker.vault` with trusted-application ACLs. Existing installs are migrated onto the generic namespace during normal command execution.

- `vault credential add <provider> <label> [--kind <kind>] [--env <env>]`
  Stores a new credential in Keychain. This is usually the first command you run for any provider.
- `vault credential list`
  Lists all stored credentials and their UUIDs. Use this when you forgot a credential ID for later commands.
- `vault credential enable <id>`
  Re-enables a previously disabled credential.
- `vault credential disable <id>`
  Disables a credential without deleting it, so profiles and connectors stop using it.
- `vault credential remove <id> --yes`
  Permanently removes the credential metadata and its Keychain secret.

Typical use:

```bash
vault credential add openai work-main --kind api_key --env work
vault credential list
```

### `vault profile`

Use `vault profile` for the compatibility baseline. A profile groups provider bindings for `vault run` and for older flows that still think in provider-level access instead of capability-level access.

- `vault profile create <name>`
  Creates an empty profile.
- `vault profile list`
  Lists all profiles.
- `vault profile show <name>`
  Shows one profile and all of its bindings.
- `vault profile bind <profile> <provider> <credential-id> --mode <inject|proxy|either>`
  Binds a stored credential to a provider inside the profile.

Mode meanings:

- `inject`: env-injection compatibility only
- `proxy`: prefer brokered HTTP forwarding when the tool can talk to the local vault
- `either`: mixed transitional mode during migration

Typical use:

```bash
vault profile create coding
vault profile bind coding openai <credential-id> --mode inject
vault profile show coding
```

### `vault run`

Use `vault run` only when a tool still expects credentials in the child-process environment. This is a user-operated compatibility path, not the recommended agent-security path.

`vault run --profile <profile> [--agent <agent>] [--project <project>] -- <command>...`

What it does:

- resolves all `inject` and `either` bindings from the profile
- reads the secrets from Keychain
- injects provider env vars such as `OPENAI_API_KEY`
- injects vault metadata such as `VAULT_PROFILE`, `VAULT_AGENT`, `VAULT_LEASE_TOKEN`, `VAULT_PROJECT`
- records a launch event for auditing

Typical use:

```bash
vault run --profile coding --agent legacy-tool --project my-app -- python main.py
```

### `vault serve`

Use `vault serve` to run the local HTTP server that backs the dashboard and brokered proxy surfaces.

- `vault serve`
  Starts the server in the foreground.
- `vault serve --background`
  Starts the server in the background and writes a PID file next to the active database.
- `vault serve --port <port>`
  Overrides the default port `8765`.
- `vault serve status`
  Checks whether the background server is running.
- `vault serve stop`
  Stops the background server.

Typical use:

```bash
vault serve --background
vault serve status
```

### `vault ui`

Use `vault ui` to open the browser dashboard. If the server is not already running, `vault ui` starts it in the background first.

`vault ui`

The dashboard includes Home, Credentials, Profiles, Stats, and Sessions pages, uses PIN-based auth, and never returns raw secrets to the browser.

### `vault stats`

Use `vault stats` to inspect aggregated provider usage.

- `vault stats`
  Human-readable text output.
- `vault stats --json`
  Machine-readable JSON array.
- `vault stats --provider <provider>`
  Restricts the output to one provider.

Typical use:

```bash
vault stats
vault stats --provider openai
vault stats --json
```

### `vault upgrade`

Use `vault upgrade` to self-update from GitHub Releases on macOS.

- `vault upgrade --check`
  Compares your installed version with the latest release without downloading artifacts.
- `vault upgrade --dry-run`
  Downloads release metadata and verifies signatures and checksums without replacing the current binary.
- `vault upgrade`
  Performs the full upgrade.
- `vault upgrade --to <version> --force`
  Installs a specific version, including rollback or same-version reinstall when you explicitly opt in.

`vault upgrade` refuses to run while a background `vault serve` daemon is active. Stop it first with `vault serve stop`.

### Phase 1 broker commands

These commands introduce the broker-native domain model:

```text
credential -> connector -> capability -> grant -> bundle -> session
```

`credential` stores the real secret. `connector` says how to talk to the upstream API. `capability` names an allowed action. `grant` gives an agent permission to use that capability. `bundle` groups grants into a workflow package. `session` is the short-lived runtime token the broker issues.

### `vault connector`

Use `vault connector` to register an upstream API connection backed by an existing credential.

- `vault connector add <name> --provider <provider> --credential <credential-id> [--base-url <url>]`
  Creates a connector.
- `vault connector list`
  Lists all connectors.
- `vault connector show <name>`
  Shows the connector details.
- `vault connector enable <name>`
  Enables a connector.
- `vault connector disable <name>`
  Disables a connector.
- `vault connector remove <name> --yes`
  Removes a connector and its capabilities.

Typical use:

```bash
vault connector add work-openai --provider openai --credential <credential-id>
vault connector show work-openai
```

### `vault capability`

Use `vault capability` to define named actions that a connector exposes.

- `vault capability add <name> --connector <connector> [--description <text>]`
  Adds one named capability to a connector.
- `vault capability list [--connector <connector>]`
  Lists all capabilities, optionally filtered to one connector.
- `vault capability remove <id> --yes`
  Removes a capability by UUID.

Typical use:

```bash
vault capability add openai.responses.create --connector work-openai \
  --description "Create model responses"
vault capability list --connector work-openai
```

Today the capability name is user-defined. It is best treated as a stable broker action label such as `openai.responses.create`, `telegram.sendMessage`, or `github.issues.create`. It is not automatically inferred from the upstream provider, and you do not need to know the exact upstream endpoint to create it. Over time, presets and import flows should generate most capability names for you.

### `vault grant`

Use `vault grant` to authorize an agent identity to use a capability.

- `vault grant add --agent <agent> --capability <capability-id> [--ttl <minutes>] [--max-requests <n>] [--confirm] [--expires-days <days>]`
  Creates a grant with optional limits and confirmation requirements.
- `vault grant list [--agent <agent>]`
  Lists grants, optionally filtered to one agent.
- `vault grant revoke <id> --yes`
  Deletes a grant.

Typical use:

```bash
vault grant add --agent codex --capability <capability-id> --ttl 60 --max-requests 100
vault grant list --agent codex
```

### `vault bundle`

Use `vault bundle` to group grants into one named workflow package. This is the concept that existing profiles will gradually evolve into.

- `vault bundle create <name> [--description <text>]`
  Creates an empty bundle.
- `vault bundle list`
  Lists bundles.
- `vault bundle show <name>`
  Shows one bundle and its attached grants.
- `vault bundle add-grant <bundle> <grant-id>`
  Adds a grant to a bundle.
- `vault bundle from-profile <profile> [--name <name>] --yes`
  Converts an existing profile into a bundle as a compatibility bridge.
- `vault bundle remove <name> --yes`
  Removes a bundle.

Typical use:

```bash
vault bundle create coding-bundle --description "Main coding workflow"
vault bundle add-grant coding-bundle <grant-id>
vault bundle show coding-bundle
```

### `vault session`

Use `vault session` to issue and inspect short-lived runtime authorization tokens scoped to bundles.

- `vault session issue --bundle <bundle> --agent <agent> [--project <project>] [--ttl <minutes>]`
  Issues a new session token.
- `vault session list`
  Lists active sessions.
- `vault session revoke <id>`
  Revokes a session by UUID.

Typical use:

```bash
vault session issue --bundle coding-bundle --agent codex --project credential-broker --ttl 30
vault session list
```

Today this command creates a broker primitive and future-facing authorization object, but the current `/v1/proxy/...` HTTP surface still expects `x-vault-lease-token`, not a session token.

### Brokered HTTP access via `vault serve`

When a tool can talk to the local vault directly, prefer brokered HTTP access instead of `vault run`.

```bash
vault serve --background

curl -X POST http://127.0.0.1:8765/v1/proxy/openai/v1/chat/completions \
  -H "x-vault-lease-token: <token>" \
  -H "content-type: application/json" \
  -d '{"model": "gpt-4", "messages": [{"role": "user", "content": "hello"}]}'
```

In this path the agent or tool sends requests to the local broker, the broker injects the real key server-side, and telemetry is recorded centrally.

Current limitation: this HTTP surface still authenticates with a lease token. In practice that means a caller must already have a valid `x-vault-lease-token`, so this is not yet a complete MCP-native or session-native agent interface.

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
| `vault-core` | Domain types (`Credential`, `Profile`, `Lease`, `Connector`, `Capability`, `Grant`, `Bundle`, `Session`), the `ProviderAdapter` trait, `VaultError` |
| `vault-db` | SQLite persistence via sqlx (credentials, profiles, bindings, leases, usage events, UI sessions, connectors, capabilities, grants, bundles, sessions) |
| `vault-secrets` | `SecretStore` trait + macOS Keychain implementation with trusted-app ACLs |
| `vault-providers` | Provider adapters (env mapping, upstream URLs, usage parsing) |
| `vault-policy` | Lease issuance, session issuance (UUID + blake3 hash), grant validation, and environment policy enforcement |
| `vault-telemetry` | Usage event recording and rollup queries |
| `vaultd` | Axum HTTP server: dashboard pages, auth, SSE, proxy routes (now a library crate) |

This describes the current implementation layout.
For the higher-level product direction, trust boundary, and target model of brokered capabilities over raw secret injection, see [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md).

## Data Storage

- **Secrets**: macOS Keychain (never in files or database)
- **Metadata**: SQLite in the workspace state directory for source builds (`/path/to/credential-broker/.local/vault.db`); installed binaries fall back to `~/.local/share/credential-broker/vault.db`
- **PID file**: stored next to the active database as `<state-dir>/vault.pid`
- **Override**: set `VAULT_DATABASE_URL` to move both the SQLite file and the state directory that holds `vault.pid`

Migrations are auto-applied on first connection.

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `VAULT_DATABASE_URL` | Override SQLite path (default: source builds use workspace `.local/vault.db`, installed binaries use `~/.local/share/credential-broker/vault.db`; `vault.pid` follows the same parent directory) |
| `VAULT_DEBUG_RUN` | Set to `1` for debug logging in `vault run` |
| `VAULT_TRUSTED_APP_PATHS` | Colon-separated extra executable paths for Keychain ACL recovery |

## Development

```bash
cargo build                          # build all crates
cargo test                           # run all tests
cargo clippy --workspace --all-targets -- -D warnings  # lint
cargo fmt --all                      # format
cargo run -p vault-cli -- <subcmd>   # run CLI
vault serve                          # start server on 127.0.0.1:8765
```

Requires:
- Rust stable toolchain (edition 2024)
- macOS (for Keychain integration)

Cutting a release: see [docs/RELEASE.md](./docs/RELEASE.md).

## Security Model

- Secrets are stored in macOS Keychain with trusted-application ACLs, not in files
- Keychain reads are non-interactive — unauthorized access fails fast with actionable error messages
- The `security` CLI is invoked by absolute path (`/usr/bin/security`) to prevent PATH hijacking
- Secrets are piped via stdin (not CLI arguments) to avoid process-list exposure
- Leases are time-bounded (default 60 minutes) with blake3-hashed tokens
- Broker sessions are capped at 1 week, scoped to bundles, and only include active grants
- Session tokens are wrapped in `Zeroizing<String>` and never stored in plain heap allocations
- Production credentials are blocked by default unless `allow_prod` is explicitly set
- Dashboard uses PIN-based auth with per-session CSRF tokens and strict CORS

## Roadmap

The phased rollout plan lives in [docs/plans/2026-04-15-capability-broker-phase-plan.md](./docs/plans/2026-04-15-capability-broker-phase-plan.md). Phase 0 (compatibility baseline) and Phase 1 (broker core domain model) are complete. Phase 1.1 (agent-access bridge) is next, followed by Phase 2 (model gateway). Other candidate work (Linux port, code signing, Homebrew tap, more provider adapters) is parked in [docs/ROADMAP.md](./docs/ROADMAP.md).

## License

MIT
