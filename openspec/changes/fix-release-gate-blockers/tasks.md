## 1. Keychain read path — remove regressing guard (UAT-FIND-001)

- [x] 1.1 Delete `let _interaction_lock = SecKeychain::disable_user_interaction()…` block in `crates/vault-secrets/src/keychain.rs::SecretStore::get`
- [x] 1.2 Remove the now-unused `use security_framework::os::macos::keychain::SecKeychain;` import
- [x] 1.3 Replace the deleted block with a `// NOTE:` comment pointing at the `keychain-acl-rewrite` follow-up change and the `audit-hardening` spec's new "known gap" requirement
- [x] 1.4 `cargo build -p vault-cli` — clean (no unused-import warning, no `SecKeychain` reference)

## 2. Dashboard SSE — push credential-state transitions (UAT-FIND-005)

- [x] 2.1 Read `crates/vaultd/src/routes/events.rs` to identify the SQLite-polling tick loop
- [x] 2.2 Add a `fetch_max_credential_updated_at` (or equivalent monotonic-marker) helper alongside the existing `fetch_max_event_time`
- [x] 2.3 In `events_handler`, track the credential-marker between ticks and emit an SSE message of type `credential` when the marker advances
- [x] 2.4 Ensure `crates/vaultd/src/routes/dashboard.rs` + `credential_row.html` template render the row with an `id` / `hx-swap-oob` pair that the client-side htmx handler can target
- [x] 2.5 Confirm the htmx `sse-swap` (or equivalent) binding on `/credentials` page subscribes to event type `credential`
- [x] 2.6 Manual verification: `vault serve --background`; open `/credentials` in a browser; `vault credential disable <id>`; row flips ≤ 4 s without manual refresh — user confirmed

## 3. Regression guard — UAT-SEC-004

- [x] 3.1 Add new `#### UAT-SEC-004 — Keychain read path silent-failure regression guard` entry to `docs/UAT.md` under `### Security regression`, type `[AUTO:ANY]`, `Cap: audit-hardening (UAT-FIND-001 regression guard)`
- [x] 3.2 The entry's `Cmd:` SHALL perform a `vault run --profile <seed-profile> --agent uat-sec-004 -- env | grep OPENAI_API_KEY` against a seeded `-A` credential; `Pass:` regex `OPENAI_API_KEY=\S+`
- [x] 3.3 Verify `grep -c '^#### UAT-' docs/UAT.md` stays within the 20–35 window defined by the `uat-release-gate` spec (current: 24)

## 4. Findings registry updates

- [x] 4.1 Flip `UAT-FIND-001` status to `Fixed (commit <sha>)` in `docs/uat-runs/FINDINGS.md` once the commit lands; update Index table severity column notation (SHA backfill pending task 7 commit)
- [x] 4.2 Flip `UAT-FIND-005` status to `Fixed (commit <sha>)` in `docs/uat-runs/FINDINGS.md` (SHA backfill pending task 7 commit)
- [x] 4.3 Do NOT edit `UAT-FIND-002` / `UAT-FIND-003` / `UAT-FIND-004` / `UAT-FIND-006` — deferred per proposal Out-of-scope

## 5. Test suite regression check

- [x] 5.1 `cargo test --workspace --quiet` reports `test result: ok` in every crate; no `FAILED`; total passing ≥ 73 (matches UAT-SEC-002 baseline) — verified 73 pass, 0 fail
- [x] 5.2 `cargo clippy --workspace --all-targets -- -D warnings` clean

## 6. UAT re-walk on the fixed binary

- [x] 6.1 Produce a new run-log at `docs/uat-runs/2026-04-14-v0.1.1-post-fix.md` (or next available date) using the template in `docs/UAT.md`
- [x] 6.2 Walk UAT-CLI-002 / 003 / 004 (golden path) — each MUST report `PASS` on a clean checkout without any working-tree patches — PASS (no patches)
- [x] 6.3 Walk UAT-DASH-003 — MUST report `PASS` (row flips ≤ 4 s) — PASS (user confirmed)
- [x] 6.4 Re-run UAT-SEC-002 (cargo test workspace) — PASS (73 tests)
- [x] 6.5 Run UAT-SEC-004 — PASS (`OPENAI_API_KEY=<non-empty>` injected)
- [x] 6.6 Compute gate per `docs/RELEASE.md` step 5: expect `Gate: PASS` — PASS

## 7. Commit + push

- [ ] 7.1 Single atomic commit: `fix(v0.1.1): restore vault run on macOS 15 + dashboard SSE credential push`
- [ ] 7.2 `git push origin main`

## 8. Archive + spec sync

- [ ] 8.1 `/opsx:archive fix-release-gate-blockers` — promotes the `audit-hardening` delta into canonical, moves the change under `openspec/changes/archive/`
- [ ] 8.2 Verify `openspec/specs/audit-hardening/spec.md` now contains the "Keychain read path silent-failure invariant" requirement with its three scenarios

## 9. Tag v0.1.1

- [ ] 9.1 Walk `docs/RELEASE.md` checklist steps 1 → 10 against the fixed main
- [ ] 9.2 On step 5 UAT pass, cite the post-fix run-log from task 6.1
- [ ] 9.3 Tag v0.1.1 per step 9; push the tag
- [ ] 9.4 `curl | bash | ~/.local/bin/vault --version` post-tag — expect `vault 0.1.1` (closes UAT-FIND-007)
