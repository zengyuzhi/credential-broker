## Context

The Codex adversarial review found 4 design flaws in the `add-web-dashboard` change. All are design-level fixes — no code exists yet, so we're updating the specs and design artifacts before implementation begins.

## Goals / Non-Goals

**Goals:**
- Make the auth boundary daemon-owned (not shared-DB-writable)
- Make brute-force protection implementable with per-challenge tracking
- Add real CSRF protection beyond SameSite cookies
- Make SSE work for CLI-originated events via cross-process mechanism

**Non-Goals:**
- Implementing the dashboard (separate change)
- Adding HTTPS/TLS
- Multi-user or remote access

## Decisions

**1. `vault ui` requests PIN from vaultd via HTTP, not direct DB write**

Current (broken): CLI generates PIN → writes hash to SQLite → any local process can do the same.

Fixed: CLI calls `POST /api/auth/challenge` on vaultd → vaultd generates PIN + challenge ID → returns `{ challenge_id, pin }` → CLI prints PIN and opens browser with `?challenge=<id>`.

Rationale: Only vaultd can create valid challenges. A local attacker can't forge one by writing to SQLite — they'd need to call the HTTP endpoint, which vaultd can rate-limit and audit.

**2. Challenge ID for per-challenge attempt tracking**

The login form submits both `challenge_id` and `pin`. The server tracks attempts per challenge (not globally). After 5 failures on a challenge, that challenge is burned.

Browser flow:
```
GET /login?challenge=<uuid>  →  renders form with hidden challenge_id field
POST /api/auth/login { challenge_id, pin }  →  validates, sets cookie
```

**3. Per-session CSRF token on all mutating routes**

Every authenticated session gets a random CSRF token stored server-side. The token is rendered into a `<meta>` tag on each page. htmx includes it as a header (`X-CSRF-Token`) on every POST/PUT/DELETE via `hx-headers`.

Server validates: cookie (session) + CSRF header (proves the request came from our rendered page, not a cross-origin form submission).

**4. SQLite polling for SSE instead of in-process broadcast**

vaultd polls the DB every 2 seconds for changes:
- `usage_events` table: `MAX(created_at)` compared to last known timestamp
- `credentials` table: check `updated_at` for state changes
- `leases` table: check for new/expired leases

This is simple, works across processes (CLI writes → vaultd reads), and avoids complex IPC. The 2-second polling interval is acceptable for a local dashboard.

Alternative considered: SQLite `sqlite3_update_hook` — rejected because sqlx doesn't expose it, and it only works within the same connection.

## Risks / Trade-offs

- [Risk] 2-second polling adds slight DB load → Mitigation: queries are simple aggregates on indexed columns; negligible for a local tool
- [Risk] Challenge endpoint could be spammed → Mitigation: rate-limit `POST /api/auth/challenge` to 3 per minute per IP
