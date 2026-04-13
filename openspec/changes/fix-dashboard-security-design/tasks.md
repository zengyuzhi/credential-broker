## Tasks

### Task 1: Update add-web-dashboard proposal with security fixes

- [x] Update `openspec/changes/add-web-dashboard/proposal.md`: change "vault ui generates PIN and stores in DB" to "vault ui requests PIN from vaultd via HTTP"
- [x] Add CSRF token requirement to the security section
- [x] Add note about SQLite polling for SSE instead of broadcast channel
- [x] Commit: `docs: update web dashboard proposal with security design fixes`

---

### Task 2: Update add-web-dashboard design with new auth architecture

- [x] Update `openspec/changes/add-web-dashboard/design.md`: replace PIN auth section with challenge-based flow (CLI → POST /api/auth/challenge → vaultd mints PIN + challenge ID)
- [x] Add CSRF token design decision (per-session token, rendered in meta tag, sent as X-CSRF-Token header)
- [x] Replace SSE broadcast channel section with SQLite polling design (2-second interval, timestamp comparison)
- [x] Add challenge rate limiting design (3 per minute)
- [x] Commit: `docs: redesign dashboard auth and SSE per security review`

---

### Task 3: Replace add-web-dashboard auth specs with hardened versions

- [x] Replace `openspec/changes/add-web-dashboard/specs/dashboard-auth/spec.md` with the fixed version from this change's delta specs
- [x] Verify challenge ID is required in all login scenarios
- [x] Verify CSRF token requirement exists for all mutating routes
- [x] Verify Origin header validation is specified
- [x] Commit: `docs: harden dashboard auth specs with challenge ID and CSRF`

---

### Task 4: Replace add-web-dashboard SSE specs with polling-based version

- [x] Replace `openspec/changes/add-web-dashboard/specs/dashboard-sse/spec.md` with the fixed version from this change's delta specs
- [x] Verify CLI-originated mutations are covered in scenarios
- [x] Verify polling interval (2s) and max latency (4s) are specified
- [x] Commit: `docs: fix SSE specs to use cross-process polling`

---

### Task 5: Replace add-web-dashboard vault-ui-command specs

- [x] Replace `openspec/changes/add-web-dashboard/specs/vault-ui-command/spec.md` with the fixed version from this change's delta specs
- [x] Verify CLI sends HTTP request to vaultd instead of writing to DB
- [x] Verify challenge ID is included in browser URL
- [x] Commit: `docs: fix vault ui command to request challenge from daemon`

---

### Task 6: Update add-web-dashboard tasks to match new design

- [x] Update Task 1 (migrations): add `challenge_id TEXT`, `csrf_token TEXT` to `ui_sessions` table
- [x] Update Task 2 (auth): add `POST /api/auth/challenge` endpoint, add challenge ID validation, add CSRF middleware
- [x] Update Task 8 (SSE): replace broadcast channel with SQLite polling loop
- [x] Update Task 9 (vault ui): CLI calls `POST /api/auth/challenge` instead of writing DB directly
- [x] Update Task 10 (security): add Origin header validation, CSRF token verification test
- [x] Commit: `docs: update web dashboard tasks to match hardened security design`
