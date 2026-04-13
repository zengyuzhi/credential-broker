## Context

vaultd already runs on `127.0.0.1:8765` with axum, serving JSON APIs (`/health`, `/stats/providers`, `/v1/proxy/...`). Adding a web dashboard means serving HTML pages alongside the existing API. The dashboard needs authentication since it exposes credential metadata and usage data.

Current vaultd state: `AppState` holds `Store` (SQLite) and `reqwest::Client`. Routes are in `routes/{health,stats,proxy}.rs`. No HTML templating, no static file serving, no session management.

## Goals / Non-Goals

**Goals:**
- Serve a functional dashboard from vaultd without external dependencies (no npm, no node)
- PIN-based auth that's fast and secure for a local tool
- Real-time updates so the dashboard stays current without manual refresh
- Clean separation between API routes (`/api/*`, `/v1/*`) and UI routes (`/`, `/credentials`, etc.)

**Non-Goals:**
- Single-page application architecture (server-rendered is simpler and more secure)
- Custom CSS framework (use Pico CSS for zero-effort styling)
- Full CRUD from browser (credential add/remove stays CLI-only)

## Decisions

**1. Server-rendered HTML with htmx over SPA (React/Vue)**

Rationale: No build step, no npm dependency, no client-side XSS surface from templating. htmx (14KB) handles dynamic updates via HTML-over-the-wire. Matches the Rust-native philosophy. The dashboard is 5 screens — SPA is over-engineering.

Alternative considered: Leptos (Rust WASM) — rejected because the ecosystem is immature and adds compile complexity. React SPA — rejected because it adds node/npm and a separate build pipeline.

**2. Askama for HTML templating**

Rationale: Compile-time checked templates, zero runtime overhead, Jinja2-like syntax. Well-maintained and widely used in the axum ecosystem. Templates live alongside Rust code as `.html` files.

Alternative considered: Maud (macro-based) — rejected because HTML-in-Rust-macros is harder to read and edit. Tera — rejected because runtime template loading adds failure modes.

**3. Daemon-owned PIN challenges with per-session CSRF tokens**

Rationale: `vault ui` requests a challenge from vaultd via `POST /api/auth/challenge` — vaultd generates the PIN and challenge ID, stores the hash. This ensures only vaultd can mint valid login challenges (a local attacker can't forge one by writing to SQLite). Each challenge has a unique ID submitted alongside the PIN for per-challenge attempt tracking (5 attempts then burned). The challenge endpoint is rate-limited to 3/minute.

Sessions include a random CSRF token rendered into a `<meta>` tag. htmx sends it as `X-CSRF-Token` header on all POST/PUT/DELETE. This blocks cross-site form submissions from other localhost apps (which `SameSite=Strict` alone cannot prevent since it's site-based, not origin-based).

Cookie attributes: `httpOnly` (no JS access), `SameSite=Strict`, `Path=/` (all routes). All mutating routes also validate `Origin` header matches `http://127.0.0.1:8765`.

Alternative considered: CLI writing PIN hash directly to shared SQLite — rejected because any same-user process with DB write access could forge challenges. Token in URL — rejected because it leaks in browser history.

**4. SSE via SQLite polling over in-process broadcast**

Rationale: SSE is unidirectional (server→client), natively supported by htmx via `hx-ext="sse"`. The event source is SQLite polling every 2 seconds (comparing `MAX(created_at)` / `updated_at` timestamps) rather than an in-process broadcast channel. This is necessary because CLI commands (`vault run`, `vault credential enable/disable`) write directly to SQLite from a separate process — an in-memory channel in vaultd would never see those mutations.

Alternative considered: In-process `tokio::sync::broadcast` — rejected because it can't observe CLI-originated DB writes. `sqlite3_update_hook` — rejected because sqlx doesn't expose it.

**5. Static assets embedded in binary via `include_str!` / `include_bytes!`**

Rationale: Single binary deployment — no need to manage a `static/` directory at runtime. htmx.min.js (~14KB) and pico.min.css (~10KB) are small enough to embed. Templates are compiled in by askama.

Alternative considered: Serving from filesystem — rejected because it complicates distribution and path resolution.

## Risks / Trade-offs

- [Risk] htmx learning curve for contributors unfamiliar with HTML-over-the-wire → Mitigation: htmx is 14 attributes, not a framework; link to docs in code comments
- [Risk] Embedded assets make the binary larger (~30KB) → Mitigation: negligible for a CLI tool
- [Risk] SSE connection stays open per browser tab → Mitigation: limit to 1 SSE connection per session, timeout after idle
- [Risk] PIN displayed in terminal could be shoulder-surfed → Mitigation: PIN is one-use and expires in 5 minutes; acceptable for local dev use
