## 1. Scaffold doc + directory

- [x] 1.1 Create `docs/uat-runs/` (empty directory placeholder via `.gitkeep`)
- [x] 1.2 Create `docs/UAT.md` skeleton: title, preamble explaining UAT's purpose, link to `uat-release-gate` canonical spec

## 2. Tag taxonomy + entry-format sections

- [x] 2.1 Add `## Tag taxonomy` section defining the 4 tags (`[AUTO:ANY]`, `[AUTO:CI]`, `[MANUAL:SHELL]`, `[MANUAL:USER]`) with one-sentence semantics each
- [x] 2.2 Add `## Entry format` section showing the 5-field bullet block template with one example AUTO entry + one example MANUAL entry
- [x] 2.3 Add `## Pass criteria` section explaining regex vs free-text rules, including the `exit 0` degenerate form
- [x] 2.4 Add `## Paid-provider gating` section documenting `$UAT_ALLOW_PAID` and the `EstCost:` field

## 3. AI-agent guideline section

- [x] 3.1 Add `## Running as an AI agent` section with the ordered runbook (read doc → execute AUTO:ANY → skip AUTO:CI → prompt for MANUAL → write run-log → report gate)
- [x] 3.2 Enumerate forbidden actions (fake PASS, silent skip, invented IDs, unauthorized paid runs, in-run doc edits)
- [x] 3.3 Document the run-log file-naming `-ai` suffix convention
- [x] 3.4 Cross-reference AI-agent tool-call patterns: use `Bash` for AUTO:ANY execution, `AskUserQuestion` (or equivalent) for MANUAL prompts

## 4. First-pass UAT entries — golden path

- [x] 4.1 `UAT-CLI-001` `[AUTO:ANY]` — `vault --version` reports version (cap: `cli-help-text`)
- [x] 4.2 `UAT-CLI-002` `[MANUAL:SHELL]` — `vault credential add <provider> <label>` round-trips a secret through keychain (cap: macOS-Keychain path)
- [x] 4.3 `UAT-CLI-003` `[MANUAL:SHELL]` — `vault profile create` + `vault profile bind` wires a credential to a profile (cap: profile + binding)
- [x] 4.4 `UAT-CLI-004` `[MANUAL:SHELL]` — `vault run --profile <name> -- env | grep <API_KEY_NAME>` proves inject works end-to-end (cap: `vault run`)

## 5. First-pass UAT entries — CLI surface

- [x] 5.1 `UAT-CLI-005` `[AUTO:ANY]` — `vault --help` tree contains every top-level subcommand
- [x] 5.2 `UAT-CLI-006` `[AUTO:ANY]` — `vault stats --json` on a pre-populated DB returns a valid JSON array
- [x] 5.3 `UAT-CLI-007` `[AUTO:ANY]` — `vault credential list --json` shape sanity check

## 6. First-pass UAT entries — dashboard

- [x] 6.1 `UAT-DASH-001` `[MANUAL:USER]` — dashboard login with correct PIN reaches the home page within 1 second (cap: `dashboard-auth`)
- [x] 6.2 `UAT-DASH-002` `[MANUAL:USER]` — wrong PIN 5 times burns the challenge; 6th attempt gets "Too many attempts" (cap: `dashboard-auth`)
- [x] 6.3 `UAT-DASH-003` `[MANUAL:USER]` — CLI `vault credential disable` surfaces on Credentials page within 4 seconds (cap: `dashboard-sse`)
- [x] 6.4 `UAT-DASH-004` `[MANUAL:USER]` — Stats page renders per-provider rollup without `NaN` or `undefined` (cap: `dashboard-pages`)

## 7. First-pass UAT entries — serve lifecycle

- [x] 7.1 `UAT-SERVE-001` `[AUTO:CI]` — `vault serve --background` → `vault serve status` reports running → `vault serve stop` cleans PID file (cap: `vault-serve-lifecycle`)
- [x] 7.2 `UAT-SERVE-002` `[AUTO:ANY]` — `curl -s http://127.0.0.1:8765/health` returns 200 while server running (cap: `vault-serve`)
- [x] 7.3 `UAT-SERVE-003` `[MANUAL:USER]` — `vault ui` auto-starts server + opens browser + PIN prompt works (cap: `vault-ui-auto-start` + `vault-ui-command`)

## 8. First-pass UAT entries — proxy (paid-gated)

- [x] 8.1 `UAT-PROXY-OAI-001` `[MANUAL:USER]` `EstCost: $0.002` — real OpenAI proxy call via `vault run` returns 200 with a `"object": "list"` response shape
- [x] 8.2 `UAT-PROXY-ANT-001` `[MANUAL:USER]` `EstCost: $0.002` — real Anthropic proxy call via `vault run` returns 200 with a valid `content[].text` response
- [x] 8.3 `UAT-PROXY-TWAPI-001` `[MANUAL:USER]` `EstCost: $0.001` — real TwitterAPI proxy call returns 200 with `tweets[]` shape

## 9. First-pass UAT entries — install + migration

- [x] 9.1 `UAT-INSTALL-001` `[MANUAL:SHELL]` — `curl | bash` in a scratch shell installs `vault` and `vault --version` works (cap: `install-script`)
- [x] 9.2 `UAT-MIGRATE-001` `[AUTO:ANY]` — fresh `cargo run -p vault-cli -- stats --json` on an empty `.local/vault.db` runs all migrations clean
- [x] 9.3 `UAT-MIGRATE-002` `[AUTO:ANY]` — schema snapshot: `sqlite3 .local/vault.db '.schema usage_events'` contains `estimated_cost_micros INTEGER` and NOT `estimated_cost_usd`

## 10. First-pass UAT entries — security regression

- [x] 10.1 `UAT-SEC-001` `[MANUAL:USER]` — dashboard rejects an old session cookie with a rotated CSRF token (cap: `dashboard-auth` CSRF)
- [x] 10.2 `UAT-SEC-002` `[AUTO:ANY]` — `cargo test --workspace` reports 73+ tests passing (regression harness for zeroize + audit fixes)
- [x] 10.3 `UAT-SEC-003` `[AUTO:ANY]` — grep check: `grep -n 'unwrap_or(now)' crates/` returns nothing (SE-06 regression guard)

## 11. Persona journey section

- [x] 11.1 Add `## Persona journeys` section grouping entries into 3 flows: "First-time install & setup" (UAT-INSTALL-001 → UAT-CLI-002 → UAT-CLI-003 → UAT-CLI-004), "Existing user, new release upgrade" (UAT-MIGRATE-001 → UAT-CLI-001 → UAT-SEC-002), "Dashboard daily use" (UAT-SERVE-003 → UAT-DASH-001 → UAT-DASH-003)

## 12. Run-log template appendix

- [x] 12.1 Add `## Run-log template` appendix showing the exact markdown + YAML front-matter shape a run-log SHALL use
- [x] 12.2 Include a worked example (toy values) showing PASS / FAIL / SKIP rows and a `Gate:` verdict

## 13. RELEASE.md integration

- [x] 13.1 Edit `docs/RELEASE.md`: insert new step "4.5 UAT pass" between existing step 4 (Security audit pass) and step 5 (CHANGELOG current)
- [x] 13.2 The step text cites `docs/UAT.md` and the `uat-release-gate` capability, includes the gate thresholds (4/4 golden, ≥95% AUTO:ANY, ≥80% MANUAL:USER), and the docs-only skip carve-out with exact retrospective phrasing
- [x] 13.3 Renumber downstream steps (5→6, 6→7, 7→8, 8→9, 9→10) in RELEASE.md only if the doc currently uses explicit numbering; if it doesn't, leave alone

## 14. First run-log

- [x] 14.1 Create `docs/uat-runs/2026-04-14-v0.1.1-pre.md` as the first run-log, using the template from task 12.1
- [x] 14.2 Populate results by walking through every `[AUTO:ANY]` entry live (should take ~5 min)
- [x] 14.3 For `[MANUAL:*]` entries, either execute them interactively (preferred) or mark `SKIP` with explicit reason (e.g., `deferred to UAT v0.1.1 proper`)
- [x] 14.4 Compute gate verdict per the thresholds; record `Gate: PASS|FAIL`
- [x] 14.5 If `Gate: FAIL`: either fix blocker OR mark this run as `v0.1.1-pre-baseline` (not tied to an actual release cut) and note in front-matter `status: baseline-only`

## 15. CHANGELOG integration

- [x] 15.1 Add a `### Quality` subsection (new; create if absent) under `[Unreleased]` citing the UAT gate: "User-acceptance-test release gate formalized at `docs/UAT.md`; first run-log at `docs/uat-runs/2026-04-14-v0.1.1-pre.md`."

## 16. Verify

- [x] 16.1 `grep -c '^#### UAT-' docs/UAT.md` returns a number between 20 and 35 (per spec requirement)
- [x] 16.2 `grep -oE 'UAT-[A-Z-]+-[0-9]+' docs/UAT.md | sort | uniq -d` returns nothing (no duplicate IDs)
- [x] 16.3 Every `**Cap:**` value in `docs/UAT.md` maps to a directory name under `openspec/specs/` (allow `cli`/`macOS-Keychain path` as non-spec meta-caps)
- [x] 16.4 `docs/RELEASE.md` contains the exact phrase "UAT pass"
- [x] 16.5 Run-log file exists at expected path with valid YAML front-matter

## 17. Commit

- [x] 17.1 Single atomic commit: `docs(uat): add release-gate UAT checklist and first-pass run`
- [x] 17.2 `git push origin main`

## 18. Close the loop

- [x] 18.1 All tasks above checked
- [x] 18.2 Archive via `/opsx:archive add-uat-release-gate` — spec sync promotes `uat-release-gate` into `openspec/specs/` and appends the UAT-step requirement into `openspec/specs/release-process/spec.md`
