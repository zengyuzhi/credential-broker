## Why

The vault CLI is fully functional but requires memorizing commands and UUIDs. A visual web dashboard served from the existing vaultd daemon would let users set up credentials, monitor usage, and manage profiles through a browser — making credential-broker accessible without reading docs. This is critical for adoption before a public release.

## What Changes

- Add a PIN-authenticated web dashboard served from vaultd at `localhost:8765`
- Add `vault ui` CLI command that requests a PIN from vaultd via HTTP and opens the browser
- Add SSE endpoint for real-time stats and session updates (driven by SQLite polling for cross-process visibility)
- Serve server-rendered HTML pages via axum with htmx for interactivity
- Add session management (PIN auth → httpOnly cookie → 4h TTL)
- Dashboard screens: Home (overview), Credentials (list + enable/disable), Profiles (list + bindings), Stats (usage analytics), Sessions (active leases)
- Secrets are never sent to the browser — only masked metadata (last 4 chars)

## Capabilities

### New Capabilities

- `dashboard-auth`: PIN-based session authentication for the web dashboard (one-time PIN, rate-limited, httpOnly cookie)
- `dashboard-pages`: Server-rendered HTML pages for credentials, profiles, stats, and sessions with htmx live updates
- `dashboard-sse`: Server-Sent Events endpoint for real-time stats, lease, and credential state changes
- `vault-ui-command`: CLI command to generate a PIN and open the dashboard in the default browser

### Modified Capabilities

_(none — all new functionality, existing CLI and API behavior unchanged)_

## Impact

- **Affected crates:** `vaultd` (new routes, templates, static assets, SSE), `vault-cli` (new `ui` subcommand)
- **New dependencies:** `askama` or `maud` (HTML templating), `tokio-stream` (SSE), Pico CSS + htmx.min.js (static assets)
- **No changes to:** vault-core, vault-db, vault-secrets, vault-providers, vault-policy, vault-telemetry
- **Security surface:** New localhost web UI requires CORS strict mode, SameSite=Strict cookies, PIN rate limiting, and no-secret-in-response policy

## Security Implications

- Web UI creates a new attack surface: any local process can reach `localhost:8765`
- Mitigated by: daemon-owned PIN challenges (CLI requests via HTTP, not shared DB), per-session CSRF tokens, SameSite=Strict cookies, Origin header validation, CORS localhost-only
- Secrets are NEVER sent to the browser — API returns masked values only (e.g. `****...4f2a`)
- Credential add/remove operations are CLI-only — the dashboard is read-mostly
- Session timeout: 4 hours, then requires new PIN from terminal

## Out of scope

- Mobile-responsive layout (desktop only for now)
- HTTPS/TLS (localhost-only, no certificates needed)
- Credential creation from browser (security risk — secret input stays in CLI)
- Streaming/SSE proxy responses
- Multi-user or remote access
