# UAT — User Acceptance Test

> **Gate contract.** Defined by the `uat-release-gate` canonical spec at
> `openspec/specs/uat-release-gate/spec.md`. This file is the source of
> truth for entries; humans, shell scripts, and AI agents all read it.
> Don't split, don't mirror, don't duplicate to another location.

UAT complements (not replaces) `cargo test` and the `security-audit-baseline`
gate. Those catch correctness regressions and defensive-posture drift. UAT
catches **user-facing feature regressions** — the kind of bug that was shipped
when `vault --version` advertised a flag that was never wired.

Related:
- `openspec/specs/uat-release-gate/spec.md` — formal requirements
- `docs/RELEASE.md` step 5 — when this gate runs
- `docs/uat-runs/` — per-release run-logs

---

## Tag taxonomy

Every UAT entry carries exactly one of four `**Type:**` tags:

| Tag | Safe for AI? | Safe for shell/CI? | Needs human? | Typical case |
|---|---|---|---|---|
| `[AUTO:ANY]` | ✅ | ✅ | no | `vault --version`, `cargo test`, deterministic grep |
| `[AUTO:CI]` | ❌ | ✅ | no | `vault serve --background` + poll + stop (multi-step shell state) |
| `[MANUAL:SHELL]` | ❌ | ❌ | keyboard | keychain prompt, `rpassword` stdin, `install.sh` in scratch shell |
| `[MANUAL:USER]` | ❌ | ❌ | eyes | dashboard visuals, SSE 4-second propagation, browser behavior |

> The `:ANY`/`:CI`/`:SHELL`/`:USER` suffix dispatches the runner. AI agents
> MUST refuse `[AUTO:CI]` and `[MANUAL:*]` entries — not because those tests
> are wrong, but because a tool-call runner cannot hold the shell/browser
> state they need.

---

## Entry format

Each UAT entry is an `#### UAT-<area>-<NNN>` heading followed by a fixed
bullet block. Areas are short kebab-case labels: `cli`, `dash`, `serve`,
`proxy-oai`, `proxy-ant`, `proxy-twapi`, `install`, `migrate`, `sec`.

```
##### UAT-EXAMPLE-A — vault --version reports a semver string
- **Type:** `[AUTO:ANY]`
- **Cap:** cli-help-text
- **Cmd:** `cargo run --quiet -p vault-cli -- --version`
- **Pass:** stdout matches `^vault \d+\.\d+\.\d+\s*$`
- **AI-safe:** yes
```

```
##### UAT-EXAMPLE-M — CLI credential-disable flips dashboard row within 4s
- **Type:** `[MANUAL:USER]`
- **Cap:** dashboard-sse
- **Preconditions:** `vault serve --background` running, dashboard logged in, ≥1 enabled credential visible
- **Steps:**
  1. Terminal: `vault credential disable <id-of-visible-row>`
  2. Browser: Credentials page — do not refresh
  3. Wait up to 4s
- **Pass:** the row's status pill flips to `disabled` within 4s without manual refresh
- **AI-safe:** no — requires browser eyes
```

**Optional fields** (appear after `**Cap:**` when relevant):
- `**Preconditions:**` — what must be true before running
- `**EstCost:**` — dollar-cost estimate for paid-provider entries
- `**Evidence-scrub:**` — extra rules for redacting the evidence snippet in run-logs

---

## Pass criteria

- `[AUTO:*]` entries: `**Pass:**` is a regex the runner applies to the
  combined stdout+stderr output, OR the degenerate form `exit 0` / `exit N`
  when only the exit code matters.
- `[MANUAL:*]` entries: `**Pass:**` is free-text criteria the human
  evaluates. Be specific: "row flips within 4s" is verifiable; "looks right"
  is not.
- Any entry can resolve to `SKIP` with a documented reason (e.g.,
  `$UAT_ALLOW_PAID not set`, `keychain-locked`, `network unavailable`).
  SKIPs do not count toward pass-rate denominators.

---

## Paid-provider gating

Entries that hit a real provider API (`UAT-PROXY-*-001`) carry an
`**EstCost:**` field and gate on `$UAT_ALLOW_PAID=1`. Unless the env var
is explicitly set in the running shell, the entry is a `SKIP` with reason
`$UAT_ALLOW_PAID not set`.

```bash
# Safe default — paid entries skip
./scripts/uat.sh
# Opt in — paid entries execute; costs ~$0.005 per full sweep
UAT_ALLOW_PAID=1 ./scripts/uat.sh
```

AI agents MUST NOT set `UAT_ALLOW_PAID` on their own initiative. See
§ "Running as an AI agent" below.

---

## Running as an AI agent

You (Claude, Codex, Gemini, or any tool-using AI) have been asked to run
UAT. Follow this runbook exactly.

### Ordered steps

1. Read this entire file. Do not summarize from memory — the entries below
   are the source of truth and may have been updated since your training
   cutoff.
2. Locate the UAT-ID range: `grep '^#### UAT-' docs/UAT.md | wc -l`.
3. For each entry with `**Type:** [AUTO:ANY]`:
   a. Run the `**Cmd:**` via a Bash tool call.
   b. Apply the `**Pass:**` regex to the combined stdout+stderr.
   c. Record `PASS` / `FAIL` + the first 500 chars of output as evidence.
4. For each entry with `**Type:** [AUTO:CI]`:
   a. **Do NOT execute.** These require multi-step shell state (background
      processes, PID-file cleanup) that your tool-call model cannot hold
      safely.
   b. Record `SKIP` with reason `AI-unsafe, defer to shell runner`.
5. For each entry with `**Type:** [MANUAL:SHELL]` or `[MANUAL:USER]`:
   a. Print the entry's heading + `**Steps:**` block to the user.
   b. Ask the user to run + report `PASS` or `FAIL` + a 1-line evidence
      note.
   c. Record their response. If they decline or say "skip", record `SKIP`
      with their stated reason.
6. Compute the gate verdict:
   - Golden path (UAT-CLI-001..004): require 4/4 PASS.
   - `[AUTO:ANY]`: require ≥95% PASS rate over non-SKIP results.
   - `[MANUAL:USER]`: require ≥80% PASS rate over non-SKIP results.
   - `[AUTO:CI]` and `[MANUAL:SHELL]`: informational only.
7. Write the run-log to `docs/uat-runs/<YYYY-MM-DD>-<version>-ai.md`
   using the template in § Run-log template.
8. Report the run-log path + `Gate: PASS|FAIL` to the user in chat.

### Forbidden actions

- ❌ Claiming `PASS` on an entry whose `**Cmd:**` you did not actually run.
- ❌ Skipping a `[MANUAL:*]` entry without surfacing it to the user.
- ❌ Inventing UAT-IDs not present in this file.
- ❌ Setting `$UAT_ALLOW_PAID=1` on your own initiative. Paid entries
  resolve to `SKIP` unless the human explicitly says "yes, run paid UAT"
  in the current session.
- ❌ Modifying `docs/UAT.md` during a run — that is a spec change requiring
  `/opsx:propose`, not a run-time edit. Report drift (broken regex, stale
  command) separately.
- ❌ Approving paid entries, slash-commands (`/opsx:archive`), or release
  steps based solely on UAT output. UAT is a gate signal; gate decisions
  belong to the human maintainer.

### Tool-call conventions

- Use `Bash` (or your runtime's equivalent) for each `[AUTO:ANY]` `**Cmd:**`.
  Do not chain unrelated commands.
- Use `AskUserQuestion` (or equivalent surface) for each `[MANUAL:*]`
  entry — one at a time, with the entry heading + steps in the prompt.
- Write the run-log as a single `Write` call at the end — do not stream
  partial updates during the run.
- If a `[AUTO:ANY]` command fails with unexpected output, record `FAIL` +
  evidence; do not retry or "heal" by adjusting the command.

---

## Entries

Golden-path entries (UAT-CLI-001..004) are reserved. Everything else is
renumberable.

### CLI surface

#### UAT-CLI-001 — vault --version reports a semver string
- **Type:** `[AUTO:ANY]`
- **Cap:** cli-help-text, binary-rename
- **Cmd:** `cargo run --quiet -p vault-cli -- --version`
- **Pass:** stdout matches `^vault \d+\.\d+\.\d+\s*$`
- **AI-safe:** yes

#### UAT-CLI-002 — vault credential add round-trips a secret via keychain
- **Type:** `[MANUAL:SHELL]`
- **Cap:** vault-secrets (macOS Keychain)
- **Preconditions:** macOS, login keychain unlocked, clean shell
- **Steps:**
  1. `vault credential add openai uat-test --kind api_key --env work`
  2. At the secret prompt, paste any non-empty string (e.g. `sk-uat-test-$(date +%s)`)
  3. At the macOS keychain GUI prompt (if shown), click "Always Allow"
  4. `vault credential list` — the new credential appears with `enabled=true`
  5. Cleanup: `vault credential remove <id> --yes`
- **Pass:** list shows the new row; remove succeeds without error
- **AI-safe:** no — keychain GUI + stdin prompt

#### UAT-CLI-003 — vault profile create + bind wires a credential
- **Type:** `[MANUAL:SHELL]`
- **Cap:** profile + binding
- **Preconditions:** one credential exists from UAT-CLI-002 (or prior)
- **Steps:**
  1. `vault profile create uat-profile`
  2. `vault profile bind uat-profile openai <credential-id> --mode inject`
  3. `vault profile show uat-profile` — shows the binding
  4. Cleanup: `vault profile remove uat-profile --yes` (if command exists) or leave in place
- **Pass:** show output contains the provider + credential-id + mode
- **AI-safe:** no — depends on prior UAT-CLI-002 state

#### UAT-CLI-004 — vault run injects an env var end-to-end
- **Type:** `[MANUAL:SHELL]`
- **Cap:** vault-run
- **Preconditions:** `uat-profile` from UAT-CLI-003 bound to an openai credential
- **Steps:**
  1. Ensure `OPENAI_API_KEY` is NOT set in the current shell: `unset OPENAI_API_KEY`
  2. `vault run --profile uat-profile --agent uat -- env | grep -E '^(OPENAI_API_KEY|VAULT_)'`
- **Pass:** output includes `OPENAI_API_KEY=<non-empty>` AND at least `VAULT_PROFILE=uat-profile`, `VAULT_AGENT=uat`, `VAULT_LEASE_TOKEN=<uuid-shape>`
- **AI-safe:** no — depends on prior UAT-CLI-002/003 state + keychain ACL already approved

#### UAT-CLI-005 — vault --help tree lists every top-level subcommand
- **Type:** `[AUTO:ANY]`
- **Cap:** cli-help-text
- **Cmd:** `cargo run --quiet -p vault-cli -- --help`
- **Pass:** stdout contains all of `credential`, `profile`, `run`, `serve`, `ui`, `stats`
- **AI-safe:** yes

#### UAT-CLI-006 — vault stats --json returns a valid JSON array
- **Type:** `[AUTO:ANY]`
- **Cap:** stats
- **Cmd:** `cargo run --quiet -p vault-cli -- stats --json`
- **Pass:** stdout is valid JSON AND matches `^\[.*\]\s*$` (array form, possibly empty)
- **AI-safe:** yes

#### UAT-CLI-007 — vault credential list --json shape sanity
- **Type:** `[AUTO:ANY]`
- **Cap:** credential CRUD
- **Cmd:** `cargo run --quiet -p vault-cli -- credential list --json 2>&1 || true`
- **Pass:** output is either valid JSON array OR a well-formed error line (`error:` prefix). Never a panic traceback.
- **AI-safe:** yes

### Dashboard

#### UAT-DASH-001 — Correct PIN reaches home page within 1 second
- **Type:** `[MANUAL:USER]`
- **Cap:** dashboard-auth
- **Preconditions:** `vault ui` has opened a dashboard URL with a PIN printed in terminal
- **Steps:**
  1. Enter the printed 6-digit PIN into the dashboard login form
  2. Click "Log in"
- **Pass:** redirect to home page within 1s, no 5xx error, session cookie set (visible in DevTools → Application → Cookies: `vault_session`, HttpOnly, SameSite=Strict)
- **AI-safe:** no — browser eyes

#### UAT-DASH-002 — Five wrong PINs burn the challenge
- **Type:** `[MANUAL:USER]`
- **Cap:** dashboard-auth
- **Preconditions:** fresh `vault ui` session with a printed PIN
- **Steps:**
  1. Enter an incorrect 6-digit PIN; submit
  2. Repeat 4 more times (5 wrong attempts total)
  3. On the 6th attempt, enter the CORRECT PIN
- **Pass:** 6th attempt is rejected with a message citing "Too many attempts" or "Challenge burned". Correct PIN after burn does NOT log in.
- **AI-safe:** no

#### UAT-DASH-003 — CLI credential-disable flips dashboard row within 4s
- **Type:** `[MANUAL:USER]`
- **Cap:** dashboard-sse
- **Preconditions:** dashboard logged in (UAT-DASH-001), ≥1 enabled credential visible
- **Steps:**
  1. Terminal: `vault credential disable <id-of-visible-row>`
  2. Browser: Credentials page — do not refresh
  3. Wait up to 4s
- **Pass:** row status pill flips to `disabled` within 4s without manual page refresh
- **AI-safe:** no

#### UAT-DASH-004 — Stats page renders without NaN or undefined
- **Type:** `[MANUAL:USER]`
- **Cap:** dashboard-pages
- **Preconditions:** dashboard logged in, at least one usage event recorded (either real proxy call or test fixture)
- **Steps:**
  1. Navigate to the Stats page
  2. Scan the provider rollup table
- **Pass:** every cell shows either a number (possibly `0`) or an explicit `—`; no `NaN`, `undefined`, `null`, or `Infinity`
- **AI-safe:** no

### Serve lifecycle

#### UAT-SERVE-001 — serve --background starts and stops cleanly
- **Type:** `[AUTO:CI]`
- **Cap:** vault-serve-lifecycle
- **Preconditions:** no existing `.local/vault.pid`
- **Cmd:** `vault serve --background && sleep 1 && vault serve status && vault serve stop && test ! -e .local/vault.pid`
- **Pass:** exit 0 AND `status` output contains `running` AND `.local/vault.pid` absent after stop
- **AI-safe:** no — background state straddles tool calls

#### UAT-SERVE-002 — /health returns 200 while serving
- **Type:** `[AUTO:ANY]`
- **Cap:** vault-serve
- **Preconditions:** `vault serve --background` already running (prior UAT-SERVE-001 start, or maintainer started manually)
- **Cmd:** `curl -sS -o /dev/null -w "%{http_code}" http://127.0.0.1:8765/health`
- **Pass:** stdout is exactly `200`
- **AI-safe:** yes (the server is externally managed)

#### UAT-SERVE-003 — vault ui auto-starts server and opens browser
- **Type:** `[MANUAL:USER]`
- **Cap:** vault-ui-auto-start + vault-ui-command
- **Preconditions:** `vault serve stop` (no background server); default browser configured
- **Steps:**
  1. `vault ui`
  2. Observe: server start log line, PIN printed to stdout, browser tab opens to dashboard login
- **Pass:** all three observations occur within 2s of invocation; PIN is 6 digits; browser lands on login page
- **AI-safe:** no

### Proxy (paid-gated)

#### UAT-PROXY-OAI-001 — Real OpenAI proxy call returns a list
- **Type:** `[MANUAL:USER]`
- **Cap:** vault-providers (openai adapter)
- **EstCost:** `$0.000` (models endpoint is free) — first and cheapest paid smoke
- **Preconditions:** `$UAT_ALLOW_PAID=1`; a valid OpenAI key bound to a profile; `vault serve --background` running
- **Steps:**
  1. Issue a lease: `export VAULT_LEASE_TOKEN=$(vault run --profile <name> --agent uat -- bash -c 'echo $VAULT_LEASE_TOKEN')`
  2. `curl -sS -H "Authorization: Bearer $VAULT_LEASE_TOKEN" http://127.0.0.1:8765/v1/proxy/openai/v1/models | head -c 200`
- **Pass:** response starts with `{"object":"list"` AND HTTP 200
- **AI-safe:** no — paid, multi-step shell

#### UAT-PROXY-ANT-001 — Real Anthropic proxy call returns content
- **Type:** `[MANUAL:USER]`
- **Cap:** vault-providers (anthropic adapter)
- **EstCost:** `~$0.002` (1 message, haiku, 5 tokens)
- **Preconditions:** `$UAT_ALLOW_PAID=1`; valid Anthropic key bound to a profile; `vault serve --background` running
- **Steps:**
  1. Prepare payload: `{"model":"claude-haiku-4-5-20251001","max_tokens":10,"messages":[{"role":"user","content":"ping"}]}`
  2. `curl -sS -H "Authorization: Bearer $VAULT_LEASE_TOKEN" -H "Content-Type: application/json" -d @payload.json http://127.0.0.1:8765/v1/proxy/anthropic/v1/messages`
- **Pass:** response contains `"content"` array AND HTTP 200 AND dashboard Stats page shows the usage event within 4s
- **AI-safe:** no

#### UAT-PROXY-TWAPI-001 — Real TwitterAPI proxy returns tweets
- **Type:** `[MANUAL:USER]`
- **Cap:** vault-providers (twitterapi adapter)
- **EstCost:** `~$0.001`
- **Preconditions:** `$UAT_ALLOW_PAID=1`; valid TwitterAPI key bound; server running
- **Steps:**
  1. `curl -sS -H "Authorization: Bearer $VAULT_LEASE_TOKEN" 'http://127.0.0.1:8765/v1/proxy/twitterapi/twitter/user/info?userName=x'`
- **Pass:** response is JSON AND HTTP 200
- **AI-safe:** no

### Install + migration

#### UAT-INSTALL-001 — curl | bash installs a working vault binary
- **Type:** `[MANUAL:SHELL]`
- **Cap:** install-script, github-release-ci, release-process
- **Preconditions:** scratch shell; no existing `~/.local/bin/vault`; PATH does not include `cargo install` targets
- **Steps:**
  1. `PATH="/usr/bin:/bin" bash -c 'curl -fsSL https://raw.githubusercontent.com/zengyuzhi/credential-broker/main/install.sh | bash'`
  2. `~/.local/bin/vault --version`
  3. Cleanup: `rm ~/.local/bin/vault`
- **Pass:** install reports latest tag name; `--version` matches the tag's semver
- **AI-safe:** no — writes to home dir, fetches network

#### UAT-MIGRATE-001 — Fresh DB runs all migrations on first open
- **Type:** `[AUTO:ANY]`
- **Cap:** vault-db
- **Cmd:** `rm -f /tmp/uat-fresh.db && VAULT_DATABASE_URL=sqlite:///tmp/uat-fresh.db?mode=rwc cargo run --quiet -p vault-cli -- stats --json`
- **Pass:** stdout is `[]` AND exit 0 AND `/tmp/uat-fresh.db` exists after the run
- **AI-safe:** yes

#### UAT-MIGRATE-002 — usage_events schema is cost_micros, not cost_usd
- **Type:** `[AUTO:ANY]`
- **Cap:** audit-hardening (SE-09 regression guard)
- **Preconditions:** fresh DB from UAT-MIGRATE-001
- **Cmd:** `sqlite3 /tmp/uat-fresh.db '.schema usage_events'`
- **Pass:** stdout contains `estimated_cost_micros INTEGER` AND does NOT contain `estimated_cost_usd`
- **AI-safe:** yes

### Security regression

#### UAT-SEC-001 — CSRF header mismatch rejects mutating request
- **Type:** `[MANUAL:USER]`
- **Cap:** dashboard-auth (CSRF)
- **Preconditions:** dashboard logged in with a valid session
- **Steps:**
  1. Open DevTools → Network
  2. Trigger a mutating action (e.g., toggle a credential enable/disable)
  3. Replay the request with a garbled `x-csrf-token` header value
- **Pass:** replayed request returns 403 with a CSRF-rejection message
- **AI-safe:** no

#### UAT-SEC-002 — cargo test workspace passes
- **Type:** `[AUTO:ANY]`
- **Cap:** audit-hardening (broad regression), security-audit-baseline
- **Cmd:** `cargo test --workspace --quiet 2>&1 | tail -5`
- **Pass:** output contains `test result: ok` AND does NOT contain `FAILED`
- **AI-safe:** yes

#### UAT-SEC-003 — No silent unwrap_or(now) on timestamp arithmetic
- **Type:** `[AUTO:ANY]`
- **Cap:** audit-hardening (SE-06 regression guard)
- **Cmd:** `grep -rE 'checked_add_signed\([^)]*\)\.unwrap_or\(.*[nN]ow' crates/ || echo "no matches"`
- **Pass:** stdout is exactly `no matches`
- **AI-safe:** yes

---

## Persona journeys

Grouping of entries into flows. Each journey is what a *class of user*
actually does, end-to-end.

### First-time install + setup
`UAT-INSTALL-001` → `UAT-CLI-001` → `UAT-CLI-002` → `UAT-CLI-003` → `UAT-CLI-004`
— a brand-new user installs, creates their first credential, wires a
profile, and proves `vault run` injects.

### Existing user, new release upgrade
`UAT-MIGRATE-001` → `UAT-MIGRATE-002` → `UAT-CLI-001` → `UAT-SEC-002`
— an existing user's DB migrates cleanly, the new binary reports its
version, regression tests pass.

### Dashboard daily use
`UAT-SERVE-003` → `UAT-DASH-001` → `UAT-DASH-003` → `UAT-DASH-004`
— maintainer opens the dashboard, logs in, makes a CLI change, verifies
the page reflects it.

A release is "shippable" when each journey has ≥80% PASS on its entries
AND the golden path is 4/4 PASS. A failed journey is a release blocker
even if aggregate thresholds pass.

---

## Run-log template

Every UAT run produces exactly one file at
`docs/uat-runs/<YYYY-MM-DD>-<version>[-<suffix>].md`. Suffix `-ai` when an
AI agent produced it. The file MUST start with YAML front-matter and MUST
contain a summary, a results table, and a failures section.

```markdown
---
version: v0.1.1-pre
date: 2026-04-14
runner: claude-opus-4-6 (1M context)
commit_sha: <sha-at-run-time>
status: complete
---

# UAT run — v0.1.1-pre (2026-04-14)

## Summary

- Total: 22 | Pass: 18 | Fail: 0 | Skip: 4
- Golden path: 4/4 ✓
- AUTO:ANY: 10/10 (100%)
- MANUAL:USER: 4/5 (80%) — one SKIP deferred to tag-time
- Gate: PASS

## Results

| UAT-ID           | Type          | Result | Evidence                            |
|------------------|---------------|--------|-------------------------------------|
| UAT-CLI-001      | [AUTO:ANY]    | PASS   | `vault 0.1.0`                       |
| UAT-CLI-005      | [AUTO:ANY]    | PASS   | help contains all 6 subcommands     |
| UAT-DASH-001     | [MANUAL:USER] | PASS   | user confirmed via chat             |
| UAT-PROXY-OAI-001| [MANUAL:USER] | SKIP   | $UAT_ALLOW_PAID not set             |
| ...              | ...           | ...    | ...                                 |

## Failures

_(empty — no failures this run)_

## Skipped

- UAT-PROXY-OAI-001 — $UAT_ALLOW_PAID not set (paid-provider gate)
- UAT-PROXY-ANT-001 — same
- UAT-PROXY-TWAPI-001 — same
- UAT-SERVE-001 — [AUTO:CI], deferred to shell runner
```

Evidence column: keep to one line; redact raw secrets (PIN, API key,
session token) — show length or shape only (`<blake3-hex-64>`, `<pin:6>`,
`<uuid-v4>`).

A partial run (runner abandoned mid-pass) MUST set `status: partial` in
front-matter and is treated as `Gate: FAIL` by `docs/RELEASE.md` step 5.
