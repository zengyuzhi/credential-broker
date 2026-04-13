## Tasks

### Task 1: Add ui_sessions table migration and DB methods

**Files:**
- Create: `migrations/0002_ui_sessions.sql`
- Create: `crates/vault-db/src/ui_sessions.rs`
- Modify: `crates/vault-db/src/lib.rs`

- [ ] Create migration with `id TEXT PK`, `challenge_id TEXT UNIQUE`, `pin_hash TEXT`, `session_token_hash TEXT`, `csrf_token TEXT`, `attempts INTEGER DEFAULT 0`, `expires_at TEXT`, `created_at TEXT`
- [ ] Add `insert_ui_session`, `get_ui_session_by_pin_hash`, `increment_attempts`, `activate_session`, `get_session_by_token_hash`, `delete_expired_sessions` methods
- [ ] Write tests for PIN validation, attempt counting, and session lookup
- [ ] Commit: `feat: add ui_sessions table and repository`

---

### Task 2: Add PIN generation and session auth middleware

**Files:**
- Create: `crates/vaultd/src/auth.rs`
- Modify: `crates/vaultd/src/app.rs`
- Modify: `crates/vaultd/Cargo.toml`

- [ ] Implement `POST /api/auth/challenge` handler: generate PIN + challenge ID, store hash, return `{ challenge_id, pin }` (rate-limited to 3/min)
- [ ] Implement `POST /api/auth/login` handler: validate PIN + challenge ID, track attempts per challenge (burn after 5), create session with CSRF token, set httpOnly cookie
- [ ] Implement axum middleware/extractor that checks session cookie on dashboard routes
- [ ] Implement CSRF middleware: validate `X-CSRF-Token` header on all POST/PUT/DELETE routes, validate `Origin` header matches `http://127.0.0.1:8765`
- [ ] Add `tower-cookies` or manual cookie handling to Cargo.toml
- [ ] Write tests for challenge creation, PIN validation, CSRF rejection, and middleware rejection
- [ ] Commit: `feat: add challenge-based PIN auth with CSRF protection`

---

### Task 3: Add HTML templating with askama and static assets

**Files:**
- Modify: `crates/vaultd/Cargo.toml`
- Create: `crates/vaultd/templates/base.html`
- Create: `crates/vaultd/templates/login.html`
- Create: `crates/vaultd/src/static_assets.rs`

- [ ] Add `askama` and `askama_axum` to vaultd dependencies
- [ ] Create base template with Pico CSS (embedded), htmx.min.js (embedded), nav sidebar
- [ ] Create login page template with PIN input form
- [ ] Serve embedded static assets at `/static/htmx.min.js` and `/static/pico.min.css`
- [ ] Verify login page renders at `http://127.0.0.1:8765/login`
- [ ] Commit: `feat: add HTML templating and static assets`

---

### Task 4: Implement dashboard pages — Home

**Files:**
- Create: `crates/vaultd/templates/home.html`
- Create: `crates/vaultd/src/routes/dashboard.rs`
- Modify: `crates/vaultd/src/routes/mod.rs`

- [ ] Create home page template: credential count, profile count, active leases, usage chart, recent sessions
- [ ] Add `GET /` handler that queries DB for overview data and renders template
- [ ] Wire route behind session auth middleware
- [ ] Commit: `feat: add dashboard home page`

---

### Task 5: Implement dashboard pages — Credentials

**Files:**
- Create: `crates/vaultd/templates/credentials.html`
- Modify: `crates/vaultd/src/routes/dashboard.rs`

- [ ] Create credentials template: table with provider, label, env, status toggle, masked secret
- [ ] Add `GET /credentials` handler
- [ ] Add `POST /api/credentials/:id/toggle` handler for enable/disable (htmx)
- [ ] Ensure secret values are masked (last 4 chars only) in all responses
- [ ] Commit: `feat: add credentials dashboard page`

---

### Task 6: Implement dashboard pages — Profiles and Sessions

**Files:**
- Create: `crates/vaultd/templates/profiles.html`
- Create: `crates/vaultd/templates/sessions.html`
- Modify: `crates/vaultd/src/routes/dashboard.rs`

- [ ] Create profiles template: list with expandable bindings
- [ ] Create sessions template: active leases at top, expired below
- [ ] Add `GET /profiles` and `GET /sessions` handlers
- [ ] Commit: `feat: add profiles and sessions dashboard pages`

---

### Task 7: Implement dashboard pages — Stats

**Files:**
- Create: `crates/vaultd/templates/stats.html`
- Modify: `crates/vaultd/src/routes/dashboard.rs`

- [ ] Create stats template: per-provider usage table, token counts, cost, filters
- [ ] Add `GET /stats` handler with optional provider filter query param
- [ ] Wire provider dropdown filter via htmx (partial page swap)
- [ ] Commit: `feat: add stats dashboard page`

---

### Task 8: Implement SSE endpoint for live updates

**Files:**
- Create: `crates/vaultd/src/routes/events.rs`
- Modify: `crates/vaultd/src/routes/mod.rs`
- Modify: `crates/vaultd/src/app.rs`

- [ ] Add `tokio-stream` to vaultd dependencies
- [ ] Implement `GET /api/events` SSE handler with session auth
- [ ] Implement SQLite polling loop (every 2s): compare `MAX(created_at)` on `usage_events`, `updated_at` on `credentials`, new/expired rows in `leases`
- [ ] Push `event: stats`, `event: credential`, `event: lease` messages when changes detected
- [ ] Wire htmx SSE extension in templates to auto-swap updated content
- [ ] Commit: `feat: add SSE endpoint with cross-process SQLite polling`

---

### Task 9: Add `vault ui` CLI command

**Files:**
- Create: `crates/vault-cli/src/commands/ui.rs`
- Modify: `crates/vault-cli/src/commands/mod.rs`
- Modify: `crates/vault-cli/src/main.rs`

- [ ] Add `UiCommand` struct with no required args
- [ ] Implement: check vaultd is running (probe `/health`), request challenge via `POST /api/auth/challenge`, receive `{ challenge_id, pin }`, print PIN, open browser with `?challenge=<id>`
- [ ] Use `open` crate or `std::process::Command::new("open")` to launch browser on macOS
- [ ] Add help text: `about = "Open the vault dashboard in your browser"`
- [ ] Write test for PIN generation and health check failure path
- [ ] Commit: `feat: add vault ui command`

---

### Task 10: Security hardening and final verification

- [ ] Add CORS middleware: allow only `http://127.0.0.1:8765` origin
- [ ] Add `Origin` header validation on all mutating routes (reject if not `http://127.0.0.1:8765`)
- [ ] Add `X-Content-Type-Options: nosniff` header to all responses
- [ ] Verify CSRF tokens are checked on `POST /api/credentials/:id/toggle` and all other POST routes
- [ ] Audit all dashboard templates and API responses for secret leakage
- [ ] Run `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] Run `cargo test`
- [ ] Manual test: `vault ui` → enter PIN → browse all pages → verify live updates
- [ ] Commit: `chore: security hardening for web dashboard`
