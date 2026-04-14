## Context

Two unrelated HIGH-severity regressions surfaced during the first UAT walk
under the `uat-release-gate` capability (run-log:
`docs/uat-runs/2026-04-14-v0.1.1-pre.md`):

1. A hardening-pass change added `SecKeychain::disable_user_interaction()` to
   `SecretStore::get` to ensure background reads fail loudly rather than
   silently prompt. On macOS 15 + `security-framework` 3.7, that call
   returns `errSecAuthFailed` even for items whose ACL explicitly permits
   silent read by the caller, breaking every `vault run` invocation.
2. The dashboard SSE stream at `GET /api/events` polls only
   `usage_events` for change detection (see
   `crates/vaultd/src/routes/events.rs:52` — `fetch_max_event_time`). It
   never observes the `credentials` table, so a CLI-driven
   `credential disable` is invisible to an open dashboard tab until a
   manual page refresh.

Both were detected by the UAT gate design's intended mechanism —
"checklist that catches bugs a unit test can't" — on its first real run.
The gate math is currently on a knife edge (MANUAL:USER at 83%, one
margin over the 80% threshold), and the golden-path 4/4 requirement is
only met on an uncommitted working tree.

## Goals / Non-Goals

**Goals:**
- Restore `vault run` on macOS 15+ so UAT-CLI-002/003/004 pass on a clean
  checkout of `main`.
- Restore the `dashboard-sse` 4-second cross-process change-detection
  contract for credential state transitions so UAT-DASH-003 passes.
- Add one regression-guard UAT entry at `[AUTO:ANY]` level so the
  keychain-read regression never reaches a release again without
  cargo-test catching it first.
- Capture the known gap in `audit-hardening` so the capability spec stops
  lying about silent-prompt protection on the read path.

**Non-Goals:**
- Full SecItem + `kSecUseAuthenticationUINone` rewrite (tracked for
  `keychain-acl-rewrite`).
- Stdin-piping fix in `put_with_access` (UAT-FIND-002).
- Dashboard sidebar CSS fix (UAT-FIND-004).
- Stats rollup provider bucketing (UAT-FIND-006).
- Any proxy / paid-provider behavior change.

## Decisions

### Decision 1 — Keychain read: minimal removal over full rewrite

**Choice:** delete the `SecKeychain::disable_user_interaction()` call and
the now-unused `use` statement from `crates/vault-secrets/src/keychain.rs`.
Do NOT replace it with an equivalent mechanism in this change.

**Why:** the proper fix — `SecItemCopyMatching` with
`kSecUseAuthenticationUINone` in the attribute dictionary — is an API
migration that the `security-framework` 3.x bindings don't obviously
expose. Scoping that research into this change risks slipping the v0.1.1
tag. The practical regression from removing the guard is narrow (keychain
items whose ACL does NOT list the calling binary would prompt the user
instead of failing), and in the vault workload every item stored via
`put_with_access` already carries the calling binary in its ACL.

**Alternative considered:** keep the call but catch `errSecAuthFailed`
and retry without the guard. Rejected: the retry path would still pay
the `errSecAuthFailed` latency on every read, and the "retry without
guard" fallback effectively IS the "no guard" behavior — the complexity
buys nothing.

### Decision 2 — SSE fan-out: extend the polling loop, don't add a broadcast channel

**Choice:** extend the existing SQLite-polling loop in `events_handler`
to also watch a monotonic marker on the `credentials` table (e.g. max
`updated_at`, or a simple `count + max(enabled, disabled_at)` tuple).
When that marker changes between ticks, emit an SSE event whose payload
the dashboard's htmx handler uses to swap the relevant row.

**Why:** the existing polling strategy is *deliberately* cross-process —
the CLI writes to SQLite, the daemon polls SQLite, the browser gets the
event. An in-memory `tokio::sync::broadcast` would be faster but would
miss events from any mutation path that bypasses the daemon (which the
CLI currently does). Sticking with the polling model preserves that
property while extending it to a second table.

**Alternative considered:**
- Add a broadcast channel in `AppState` that mutation handlers publish
  to. Rejected: CLI-driven mutations don't go through the daemon, so the
  channel would miss them — the exact failure we're trying to fix.
- SQLite `UPDATE` hooks via `sqlx` connection events. Rejected: `sqlx`
  doesn't expose the hook API cleanly, and the hook only fires on the
  connection that made the mutation — cross-connection visibility still
  needs polling.

### Decision 3 — No canonical spec modifications

**Choice:** this change ships with zero delta specs. Verified 2026-04-14
against `openspec/specs/`:

- `audit-hardening/spec.md` contains 10 requirements covering zeroize
  discipline, lease-TTL validation, timestamp/latency overflow handling,
  microdollar cost storage, and ROADMAP retirement. None of them mandate
  `disable_user_interaction()` or equivalent silent-prompt behavior on
  the keychain read path, so removing the call does not break an
  existing requirement.
- `dashboard-sse/spec.md` already contains the scenario
  "CLI credential change appears in SSE within 4 seconds" under the
  `SSE event stream endpoint` requirement. The SSE fix brings the code
  into compliance with an existing contract — it does not change the
  contract.

**Why:** introducing spec deltas when no requirement text changes would
pollute the spec tree with cosmetic edits and degrade the signal-to-noise
ratio of `openspec diff`. The `audit-hardening` spec will be updated
when (and only when) `keychain-acl-rewrite` reinstates an explicit
invariant — that change will add the requirement at the same time it
ships the implementation that satisfies it.

**Alternative considered:** add a purely documentary "known gap"
requirement to `audit-hardening` now, even without text that constrains
behavior. Rejected: a requirement with no scenarios is unverifiable and
dilutes the spec.

### Decision 4 — Add UAT-SEC-004, not a cargo-test integration test

**Choice:** add one UAT entry at `[AUTO:ANY]` level that fetches a
`security add-generic-password -A`-created item via the `vault` binary's
keychain read path. If the regression returns, the entry fails at the
AUTO:ANY tier and blocks the gate without needing human keyboards.

**Why:** a `cargo test` integration test that touches the real Keychain
requires either a mocked keychain (which is what let the original
regression through) or CI infrastructure that grants keychain access,
which doesn't exist yet. A UAT entry runs on every pre-release walk
against the real artifact; this is where macOS-system-integration bugs
belong today.

**Alternative considered:** a dedicated integration test in
`crates/vault-secrets/tests/`. Not rejected for the long term — just
deferred to the CI work stream where keychain fixtures can be created.

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| Keychain read regresses *silently* into "prompts user instead of failing" if a user has a hand-crafted ACL that excludes vault | Document the known gap in `audit-hardening` spec; open follow-up `keychain-acl-rewrite` before v0.1.3. Practical impact today: zero, since every `put_with_access`-created item includes the calling binary. |
| SSE polling for the `credentials` table doubles the poll-cost per tick (two queries instead of one) at the current 2 s interval | Queries are on indexed columns with `MAX()` aggregation — measured cost is µs-scale. If an SSE scaling problem emerges later, migrate all markers to a `change_log` summary table in one place. |
| New UAT-SEC-004 tightens the AUTO:ANY denominator — adding a flaky entry would hurt the 95% threshold | Entry is a straight regex match on stdout of a deterministic `vault run` invocation against a pre-seeded `-A` credential; no flakiness vectors identified. |
| The `audit-hardening` spec modification could be read as "weakening is fine whenever UAT pressures the schedule" | Modified text explicitly names the follow-up `keychain-acl-rewrite` change and calls the relaxation temporary. Retro check: any future "weaken this" proposal must cite an equivalent follow-up. |

## Migration Plan

1. Apply keychain.rs edit (1-line removal + import cleanup).
2. Apply events.rs + credential_row template edit for SSE credential-state push.
3. Add UAT-SEC-004 to `docs/UAT.md` and flip UAT-FIND-001 / UAT-FIND-005
   in `docs/uat-runs/FINDINGS.md` to `Fixed` with the change's commit hash.
4. `cargo test --workspace` passes ≥ 73 tests.
5. Re-run the UAT MANUAL:USER batch from
   `docs/uat-runs/2026-04-14-v0.1.1-pre.md` (DASH-001..004, SERVE-003,
   SEC-001) against the fixed binary — expect DASH-003 to flip from FAIL
   to PASS.
6. Tag v0.1.1 per `docs/RELEASE.md` step 5 UAT pass criteria.

**Rollback:** `git revert` the change commit. Both fixes are strictly
local to `crates/vault-secrets/src/keychain.rs` and
`crates/vaultd/src/routes/events.rs` + its templates; no schema or
binary-format changes.

## Open Questions

- **Should UAT-SEC-004 live under `audit-hardening` or a new
  `keychain-read-path` cap?** Leaning `audit-hardening` since the
  regression is audit-originated, but if the new cap becomes necessary
  for the SecItem rewrite later, moving UAT-SEC-004 alongside would be
  clean.
- **Does the SSE fan-out also need to cover `profiles` / `bindings`
  mutations?** Current UAT does not require it, but the symmetry
  argument is strong. Defer to after v0.1.1 — add as a separate UAT
  entry if the user notices a similar gap in daily use.
