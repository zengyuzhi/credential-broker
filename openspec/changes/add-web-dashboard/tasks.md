## Tasks

### Task 1: Add ui_sessions table migration and DB methods

**Files:**
- Create: `migrations/0002_ui_sessions.sql`
- Create: `crates/vault-db/src/ui_sessions.rs`
- Modify: `crates/vault-db/src/lib.rs`

- [x] Create migration with `id TEXT PK`, `challenge_id TEXT UNIQUE`, `pin_hash TEXT`, `session_token_hash TEXT`, `csrf_token TEXT`, `attempts INTEGER DEFAULT 0`, `expires_at TEXT`, `created_at TEXT`
- [x] Add `insert_ui_session`, `get_ui_session_by_pin_hash`, `increment_attempts`, `activate_session`, `get_session_by_token_hash`, `delete_expired_sessions` methods
- [x] Write tests for PIN validation, attempt counting, and session lookup
- [x] Commit: `feat: add ui_sessions table and repository`

---

### Task 2: Add PIN generation and session auth middleware

**Files:**
- Create: `crates/vaultd/src/auth.rs`
- Modify: `crates/vaultd/src/app.rs`
- Modify: `crates/vaultd/Cargo.toml`

- [x] Implement `POST /api/auth/challenge` handler: generate PIN + challenge ID, store hash, return `{ challenge_id, pin }` (rate-limited to 3/min)
- [x] Implement `POST /api/auth/login` handler: validate PIN + challenge ID, track attempts per challenge (burn after 5), create session with CSRF token, set httpOnly cookie
- [x] Implement axum middleware/extractor that checks session cookie on dashboard routes
- [x] Implement CSRF middleware: validate `X-CSRF-Token` header on all POST/PUT/DELETE routes, validate `Origin` header matches `http://127.0.0.1:8765`
- [x] Add `tower-cookies` or manual cookie handling to Cargo.toml
- [x] Write tests for challenge creation, PIN validation, CSRF rejection, and middleware rejection
- [x] Commit: `feat: add challenge-based PIN auth with CSRF protection`

---

### Task 3: Add HTML templating with askama and static assets

**Files:**
- Modify: `crates/vaultd/Cargo.toml`
- Create: `crates/vaultd/templates/base.html`
- Create: `crates/vaultd/templates/login.html`
- Create: `crates/vaultd/src/static_assets.rs`

- [x] Add `askama` and `askama_axum` to vaultd dependencies
- [x] Create base template with Pico CSS (CDN), htmx.min.js (CDN), nav sidebar
- [x] Create login page template with PIN input form
- [x] Serve login page at `/login`
- [x] Verify login page renders at `http://127.0.0.1:8765/login`
- [x] Commit: `feat: add HTML templating and static assets`

---

### Task 4: Implement dashboard pages — Home

**Files:**
- Create: `crates/vaultd/templates/home.html`
- Create: `crates/vaultd/src/routes/dashboard.rs`
- Modify: `crates/vaultd/src/routes/mod.rs`

- [x] Create home page template: credential count, profile count, active leases, usage chart, recent sessions
- [x] Add `GET /` handler that queries DB for overview data and renders template
- [x] Wire route behind session auth middleware
- [x] Commit: `feat: add dashboard home page with overview stats`

---

### Task 5: Implement dashboard pages — Credentials

**Files:**
- Create: `crates/vaultd/templates/credentials.html`
- Modify: `crates/vaultd/src/routes/dashboard.rs`

- [x] Create credentials template: table with provider, label, env, status toggle
- [x] Add `GET /credentials` handler
- [x] Add `POST /api/credentials/:id/toggle` handler for enable/disable (htmx) with CSRF validation
- [x] Ensure secret values are never exposed in responses
- [x] Commit: `feat: add credentials dashboard page with enable/disable toggle`

---

### Task 6: Implement dashboard pages — Profiles and Sessions

**Files:**
- Create: `crates/vaultd/templates/profiles.html`
- Create: `crates/vaultd/templates/sessions.html`
- Modify: `crates/vaultd/src/routes/dashboard.rs`

- [x] Create profiles template: list with expandable bindings
- [x] Create sessions template: active leases at top, expired below
- [x] Add `GET /profiles` and `GET /sessions` handlers
- [x] Commit: `feat: add profiles and sessions dashboard pages`

---

### Task 7: Implement dashboard pages — Stats

**Files:**
- Create: `crates/vaultd/templates/stats.html`
- Modify: `crates/vaultd/src/routes/dashboard.rs`

- [x] Create stats template: per-provider usage table, token counts, cost, filters
- [x] Add `GET /stats` handler with optional provider filter query param
- [x] Wire provider dropdown filter via htmx (partial page swap)
- [x] Commit: `feat: add stats dashboard page with provider filter`

---

### Task 8: Implement SSE endpoint for live updates

**Files:**
- Create: `crates/vaultd/src/routes/events.rs`
- Modify: `crates/vaultd/src/routes/mod.rs`
- Modify: `crates/vaultd/src/app.rs`

- [x] Add `tokio-stream`, `async-stream`, `futures` to vaultd dependencies
- [x] Implement `GET /api/events` SSE handler with session auth
- [x] Implement SQLite polling loop (every 2s): compare `MAX(created_at)` on `usage_events`, credential count, active lease count
- [x] Push `event: stats`, `event: credential`, `event: lease` messages when changes detected
- [x] Wire native EventSource + htmx.ajax auto-refresh in base template
- [x] Commit: `feat: add SSE endpoint with cross-process SQLite polling`

---

### Task 9: Add `vault ui` CLI command

**Files:**
- Create: `crates/vault-cli/src/commands/ui.rs`
- Modify: `crates/vault-cli/src/commands/mod.rs`
- Modify: `crates/vault-cli/src/main.rs`

- [x] Add `UiCommand` struct with no required args
- [x] Implement: check vaultd is running (probe `/health`), request challenge via `POST /api/auth/challenge`, receive `{ challenge_id, pin }`, print PIN, open browser with `?challenge=<id>`
- [x] Use `std::process::Command::new("open")` to launch browser on macOS
- [x] Add help text: `about = "Open the vault dashboard in your browser"`
- [x] Write test for base URL constant
- [x] Commit: `feat: add vault ui command to open dashboard`

---

### Task 10: Security hardening and final verification

- [x] Add CORS middleware: allow only `http://127.0.0.1:8765` origin
- [x] Add `Origin` header validation on all mutating routes (via validate_csrf)
- [x] Add `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Cache-Control: no-store` headers
- [x] Verify CSRF tokens are checked on `POST /api/credentials/:id/toggle` and all other POST routes
- [x] Audit all dashboard templates and API responses for secret leakage
- [x] Run `cargo clippy --workspace --all-targets -- -D warnings` — clean
- [x] Run `cargo test` — 72 tests pass
- [ ] Manual test: `vault ui` → enter PIN → browse all pages → verify live updates
- [x] Commit: `chore: security hardening for web dashboard`
