## Why

Codex adversarial review identified 4 security design flaws in the `add-web-dashboard` proposal that must be fixed before implementation:

1. **[critical]** PIN issuance via shared SQLite lets any local process forge login challenges
2. **[high]** 5-attempt brute-force protection is unimplementable without a challenge ID
3. **[high]** `SameSite=Strict` doesn't block same-site CSRF from other localhost apps
4. **[high]** In-process SSE broadcast channel can't observe CLI-originated DB mutations

## What Changes

- Redesign PIN auth so vaultd owns challenge creation (not CLI writing to shared DB)
- Add challenge ID to the login flow for per-challenge attempt tracking
- Add per-session CSRF tokens on all mutating dashboard routes
- Replace in-process broadcast with SQLite polling for SSE event source

## Capabilities

### Modified Capabilities

- `dashboard-auth`: Redesign PIN challenge to be daemon-owned, add challenge ID, add CSRF tokens
- `dashboard-sse`: Replace broadcast channel with cross-process SQLite polling
- `vault-ui-command`: CLI requests PIN from vaultd via HTTP instead of writing to DB directly

## Impact

- **Affected artifacts:** `add-web-dashboard` proposal, design, specs (dashboard-auth, dashboard-sse, vault-ui-command), and tasks
- **No code changes** — this fixes the design before any implementation begins
- **Security posture:** Eliminates all 4 findings from the adversarial review

## Security Implications

This change IS the security fix. It addresses:
- Auth boundary collapse (shared DB → daemon-owned challenges)
- Brute-force accounting gap (anonymous → challenge-ID-tracked)
- CSRF bypass via same-site localhost (cookie-only → cookie + CSRF token)
- SSE blindness to CLI events (in-process channel → DB polling)

## Out of scope

- Implementing the dashboard (covered by `add-web-dashboard`)
- Changes to existing CLI or proxy functionality
