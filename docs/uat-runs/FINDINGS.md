# UAT Findings Registry

Rolling registry of defects, regressions, and ancillary findings surfaced by UAT
runs. Every entry carries a stable `UAT-FIND-NNN` identifier, the originating
run-log, a status, and — where resolved — the fix commit.

## Rules

- **Every FAIL** in any run-log under `docs/uat-runs/` produces at least one entry
  here. Copy the failure's symptom, root cause, and disposition into an entry.
- **Ancillary findings** (surprises noted during a PASS run — e.g. unexpected
  output shape, documentation drift, latent bugs not covered by the entry's
  Pass criterion) get entries too, at lower severity.
- **Status progression:** `Open` → `Fixed` (cite commit) or `Accepted` (cite
  rationale). Entries are append-only; never edit a `Fixed` entry's fix commit
  without a follow-up note.
- **IDs are sequential** across all runs. Don't renumber when an entry is
  resolved; the ID is a permanent pointer.
- **Severity** uses the same scale as the security audit baseline:
  `CRITICAL` / `HIGH` / `MEDIUM` / `LOW` / `INFO`. Functional-only risk (not
  security) calibrates the scale to blast radius on UX, not on data exposure.

## Index

| ID | Found | Severity | Status | Title |
|----|-------|----------|--------|-------|
| UAT-FIND-001 | 2026-04-14 | HIGH | Fixed | `disable_user_interaction()` breaks all `vault run` reads on macOS 15 |
| UAT-FIND-002 | 2026-04-14 | MEDIUM | Open | `security add-generic-password -w` stdin piping silently falls back to `/dev/tty` |
| UAT-FIND-003 | 2026-04-14 | LOW | Open | `VAULT_LEASE_TOKEN` shape is two concatenated UUIDs |
| UAT-FIND-004 | 2026-04-14 | MEDIUM | Fixed | Dashboard sidebar collapses to an illegible vertical strip on the Home page |
| UAT-FIND-005 | 2026-04-14 | HIGH | Fixed | Dashboard SSE does not push credential-state changes to the browser |
| UAT-FIND-006 | 2026-04-14 | LOW | Open | Stats page lumps `process_launch` telemetry under `provider=vault`, conflating with real usage |
| UAT-FIND-007 | 2026-04-14 | CRITICAL | Open — resolves at v0.1.1 | Released v0.1.0 binary has broken `--version`; install-script users get a crippled `vault` |

---

## UAT-FIND-001 — `disable_user_interaction()` breaks `vault run` on macOS 15

- **Found by:** `docs/uat-runs/2026-04-14-v0.1.1-pre.md` (UAT-CLI-004 initial FAIL)
- **Severity:** HIGH (breaks the `vault run` golden path for every user on
  macOS 15+; effectively renders the tool non-functional on the author's own
  machine).
- **Status:** **Fixed** via `fix-release-gate-blockers` — the
  `SecKeychain::disable_user_interaction()` guard and its import are gone
  from `crates/vault-secrets/src/keychain.rs::SecretStore::get`, replaced
  by a `// NOTE:` comment that references the `keychain-acl-rewrite`
  follow-up and the `audit-hardening` spec's "Keychain read path
  silent-failure invariant" known-gap requirement. Regression guard:
  `UAT-SEC-004`. Commit SHA to be backfilled at task 7.

### Symptom

```
Error: failed to load secret for ai.zyr1.vault/credential:<id>:api_key
Caused by:
    The user name or passphrase you entered is not correct.
```

Affects **every** `vault run` invocation, regardless of which credential is
bound. Reproduced against both a fresh `uat-test` credential and a pre-existing
`work-main` credential with healthy ACL. `/usr/bin/security find-generic-password
-w` reads the same items silently, confirming the keychain entry itself is not
corrupted.

### Root cause

`SecKeychain::disable_user_interaction()` at
`crates/vault-secrets/src/keychain.rs:141` (pre-fix) returns `errSecAuthFailed`
on macOS 15.x with `security-framework` 3.7.0, **even for items whose ACL
permits silent access** by the calling binary. The guard was introduced to make
background reads fail loudly rather than silently prompt, but its behavior on
the current macOS + security-framework combo regressed the baseline read path.

Confirmed by temporarily commenting the guard out and re-running UAT-CLI-004 —
all four required env vars then injected correctly. `cargo test --workspace`
remained at 73 passing, 0 FAILED after the change.

### Fix

Remove the `disable_user_interaction()` guard and the now-unused
`SecKeychain` import from `crates/vault-secrets/src/keychain.rs`. Document in
the same file that a non-regressing replacement is pending (candidate:
`SecItemCopyMatching` with `kSecUseAuthenticationUINone`).

### Follow-up

Open a separate proposal to restore the "no silent GUI prompt" invariant via the
SecItem API. Entry **remains visible** in this registry after fix so future
runs can cross-reference it if the regression ever recurs.

---

## UAT-FIND-002 — `security add-generic-password -w` stdin piping does not work

- **Found by:** `docs/uat-runs/2026-04-14-v0.1.1-pre.md` (diagnosis during
  UAT-CLI-004 root-cause hunt; UAT-CLI-002 itself passed via unintended
  fallback).
- **Severity:** MEDIUM (correctness + security: the code path claims to keep
  secrets out of process argument lists, but actually reads them from the
  terminal. `CLAUDE.md` gotcha documents the claimed-but-false behavior).
- **Status:** **Open.**

### Symptom

`put_with_access()` in `crates/vault-secrets/src/keychain.rs` spawns
`/usr/bin/security add-generic-password … -w` with `stdin(Stdio::piped())` and
writes the secret to the child's stdin. Observed behavior on macOS 15: the
child process **ignores the stdin pipe** and prompts interactively on
`/dev/tty` (`password data for new item:` / `retype password for new item:`).
The secret the user types at the terminal is what actually gets stored.

Reproduction:

```shell
echo -n "piped-value" | /usr/bin/security add-generic-password \
  -s ai.zyr1.vault.diag -a probe -U -w
# stdout still contains: "password data for new item:" / "retype password for new item:"
```

### Root cause

`security add-generic-password -w <password>` reads the password from the CLI
argument; `-w` with **no trailing value** prompts on `/dev/tty` regardless of
whether stdin is a pipe. The `CLAUDE.md` gotcha and the code comment both assert
the opposite; the assertion predates macOS 15 (where the behavior may have
differed) or is outright incorrect.

### Proposed fix (not yet applied)

Replace the `security(1)` subprocess invocation with the `security-framework`
crate's `set_generic_password` + an `SecAccess` ACL created via
`SecAccessCreateWithOwnerAndACL` (or equivalent 3.x-bindable API). The
subprocess path can stay as a fallback if the Rust binding is incomplete, but
the stdin claim needs to be removed from the code comment and CLAUDE.md.

### Blast radius today

- Mitigated in practice by the fact that users type secrets interactively at
  a TTY prompt, so the secret never actually hits `argv`.
- The **intent** (keep secrets out of `/proc/<pid>/cmdline`) is preserved by
  accident, but the mechanism differs from what the code claims.

### Regression guard to add later

UAT entry: launch `security add-generic-password` with piped stdin and assert
it does NOT write to `/dev/tty` (probe via `tty` file descriptor detection).
Open once a fix is ready.

---

## UAT-FIND-003 — `VAULT_LEASE_TOKEN` is two concatenated UUIDs

- **Found by:** `docs/uat-runs/2026-04-14-v0.1.1-pre.md` (UAT-CLI-004 PASS,
  ancillary observation on injected env var shape).
- **Severity:** LOW (cosmetic; loosely satisfies UAT-CLI-004 pass criterion
  "uuid-shape", but consumer code parsing the token needs to understand the
  compound format).
- **Status:** **Open.**

### Symptom

```
VAULT_LEASE_TOKEN=32ca60b7-1119-416d-bbc5-c8a5b554566fcaff9ba9-7552-43f7-9555-c5bfe8d428ed
```

72 characters = two RFC-4122 UUIDs concatenated without a separator. Expected
either one UUID (lease ID) or an explicit `<lease_id>.<raw_token>` delimited
format.

### Questions

1. Is the second UUID the raw lease token pre-blake3-hash, or something else?
2. Does `vaultd`'s lease-token parser expect the compound form, or does it
   split on the 36-char boundary implicitly? Either is fragile.
3. Should UAT-CLI-004's Pass criterion tighten from generic "uuid-shape" to a
   concrete regex once the decision is made?

### Proposed disposition

Root-cause inside `vault_policy::lease::issue_lease` and the env-var builder
in `vault-cli`. Either:
- Change the format to `<lease_id>.<raw_token>` (breaking — bump minor version).
- Keep the concatenation but document it as `<lease_id><raw_token>`, both
  fixed-width 36-char UUIDs, and publish a helper in `vault-core` for
  consumers.

Decide before v0.1.1 tag. UAT-CLI-004's regex tightens in the same change.

---

## UAT-FIND-004 — Dashboard sidebar collapses to an illegible vertical strip

- **Found by:** `docs/uat-runs/2026-04-14-v0.1.1-pre.md` (UAT-DASH-001 PASS,
  screenshot evidence from user).
- **Severity:** MEDIUM (nav is effectively unusable; user reached Home via the
  login redirect but cannot click into Credentials, Profiles, or Stats without
  resizing the window or navigating via URL bar).
- **Status:** **Fixed** via `fix-release-gate-blockers` (scope-expanded) —
  `crates/vaultd/templates/base.html` now scopes `.vault-nav ul { display: block }`
  and related block-level list overrides, restoring the vertical sidebar that
  Pico CSS's default `nav ul { display: flex }` had collapsed. Active-page
  state uses `color-mix(... var(--pico-primary) 12%, transparent)` — no gradient
  text, no left-stripe border, no glassmorphism. Commit SHA to be backfilled
  at task 7.

### Symptom

On Home (`/`), the left sidebar renders as a ~200px-wide vertical strip with
the nav labels collapsed into a single overlapping line reading
`VaultHomeCredentialsPr…` (truncated mid-word). The main content area renders
correctly (Overview cards, Provider Usage table, Recent Activity table) so the
issue is confined to the sidebar layout.

### Likely cause

CSS flex/grid miscalculation — the sidebar container has no `min-width` /
`flex-shrink: 0` so it shrinks under pressure from the main content's intrinsic
width, and the nav items render inline instead of stacked. Template area:
`crates/vaultd/templates/` (base + home partial).

### Reproduction

1. `vault serve stop && rm -f .local/vault.pid`
2. `vault ui` → log in with the printed PIN
3. Observe the sidebar on the Home page: collapsed strip instead of a
   navigable list of Home / Credentials / Profiles / Stats / Leases / Settings.

### Proposed disposition

- Set explicit `width` / `min-width` on the sidebar container.
- Confirm Pico CSS grid settings match the template expectations (`main.container`
  may be forcing the sidebar to shrink).
- Add a UAT entry `UAT-DASH-005 — Home sidebar nav is visible and clickable`
  once the fix lands, so the regression is caught next run.

### Blast radius today

UAT-DASH-003 (CLI disable → row flip) and UAT-DASH-004 (Stats page renders)
still work via direct URL navigation, but manual users without URL-bar reflexes
effectively can't move past Home.

---

## UAT-FIND-005 — Dashboard SSE does not push credential-state changes

- **Found by:** `docs/uat-runs/2026-04-14-v0.1.1-pre.md` (UAT-DASH-003 FAIL,
  computer-use screenshot evidence).
- **Severity:** HIGH (breaks the advertised "cross-process change detection
  within 4 seconds" contract for the `dashboard-sse` capability — the whole
  point of the SSE endpoint).
- **Status:** **Fixed** via `fix-release-gate-blockers` — credential
  watermark in `crates/vaultd/src/routes/events.rs` swapped from a row-count
  (blind to in-place toggles) to `MAX(updated_at)` read via the new
  `Store::max_credential_updated_at` helper. The base-template SSE
  subscription (`event: credential`) also switched from
  `htmx.ajax('GET', …, {target:'main', swap:'innerHTML'})` to a
  `fetch` + `DOMParser` extract that swaps only the `<main>` inner
  content — the old path dumped the entire returned `<html>` tree
  inside `<main>` and produced a duplicated sidebar nav on every
  toggle (rare pre-fix because `credential` only fired on add/remove).
  Commit SHA to be backfilled once the change is committed under task 7
  of the `fix-release-gate-blockers` change.

### Symptom

1. Browser on `/credentials` shows row `provider=openai label=uat-test status=✓ Enabled`.
2. Terminal: `vault credential disable 0bd5c9e2-aead-462e-b1fe-db20e5a3bb6c`
   completes successfully (stdout: `Credential 0bd5c9e2... disabled.`).
3. Wait 4 seconds without refreshing the browser.
4. Row still displays `✓ Enabled`. Verified via screenshot at
   `DELL S2725DS` at 16:56 local, >1 minute after the CLI disable.

Manual page refresh is expected to show the correct disabled state
(backend mutation is fine), confirming the failure is strictly on the SSE
push path rather than the DB write.

### Likely causes to investigate

- SSE endpoint (`GET /api/events`) may not be polling / fanning out the
  credential-state-changed event. CLAUDE.md notes polling every 2s.
- htmx swap rule for the row might not be wired — event is delivered but the
  client never re-renders.
- Browser session token mismatch; SSE dropped on auth check.
- Template may emit the row without `hx-swap-oob` / `id` so the client has
  nothing to target.

### Reproduction

```bash
vault serve --background && sleep 1
vault ui   # log in in browser, navigate to /credentials
# note a row whose credential_id you will flip
vault credential disable <that-credential-id>
# observe browser for 4s; row status pill must flip. It does not.
```

### Proposed disposition

Root-cause inside `crates/vaultd/src/handlers/events.rs` (SSE) +
`crates/vaultd/templates/credentials.html` (row template). Pair with
UAT-FIND-004 for a single "dashboard regression" change, since both
point at `crates/vaultd/templates/**`. Block any v0.1.1 tag on fix.

### Regression guard

UAT-DASH-003 already exists and now reliably catches this. Keep as golden-path
adjacent; consider promoting to the 4-item golden-path set once SSE is the
primary distribution channel.

---

## UAT-FIND-006 — Stats page buckets `process_launch` under `provider=vault`

- **Found by:** `docs/uat-runs/2026-04-14-v0.1.1-pre.md` (UAT-DASH-004 PASS,
  ancillary observation on Stats + Home Provider Usage tables).
- **Severity:** LOW (the Pass criterion for UAT-DASH-004 only checks for
  NaN/undefined/null/Infinity; bucketing is cosmetic today). Will inflate to
  MEDIUM once real proxy calls begin recording events — the `vault` bucket
  will drown the actual provider rows.
- **Status:** **Open.**

### Symptom

Both Home `Provider Usage` and Stats `Summary` display a single row whose
Provider column is `vault`, with Requests=4 tokens=0 Est. Cost=$0. Those 4
events correspond to the `vault run` process_launch telemetry the CLI emits,
NOT to actual OpenAI/Anthropic/TwitterAPI usage. Once a user flips
`$UAT_ALLOW_PAID=1` and runs the proxy UATs, the table will show `openai`
and `vault` side-by-side — confusing at best, wrong at worst.

### Proposed disposition

Two options, pick one:
1. Drop `process_launch` events from the Provider-Usage / Summary rollups
   entirely. They are CLI-internal bookkeeping, not provider usage.
2. Keep them, but label the Provider column as `vault (launch)` or put them
   in a separate "Launches" table beneath Summary.

Option 1 is the cleaner call; `process_launch` still lives in Recent Events
for audit-log purposes where context makes its meaning obvious.

### Regression guard

Extend UAT-DASH-004's Pass criterion to "provider column contains only values
from the canonical set (`openai`, `anthropic`, `twitterapi`, ...) plus an
explicit opt-in `vault (launch)`". Codify after disposition lands.

---

## UAT-FIND-007 — Released v0.1.0 binary has broken `--version`

- **Found by:** `docs/uat-runs/2026-04-14-v0.1.1-pre.md` (UAT-INSTALL-001 FAIL,
  live output from `curl | bash | ~/.local/bin/vault --version`).
- **Severity:** CRITICAL at discovery (every install-script user gets a
  crippled binary). Already resolved on `main` via commit `5990f12` "fix: wire
  up clap --version flag in vault CLI"; the next tag (v0.1.1) will ship a
  working binary.
- **Status:** **Open — resolves automatically at v0.1.1 tag.** This is the
  founding regression that motivated the entire `uat-release-gate` capability.

### Symptom

```
$ curl -fsSL https://.../install.sh | bash
Detecting latest release...
Latest version: v0.1.0
Downloading vault-aarch64-apple-darwin.tar.gz...
vault v0.1.0 installed to /Users/zengy/.local/bin/vault

$ ~/.local/bin/vault --version
error: unexpected argument '--version' found
```

The same binary's `--help` works; only `--version` errors. The installed binary
was built from the v0.1.0 tag point, which predates the clap `#[command(version)]`
wiring landed in `5990f12`.

### Why UAT caught this

The whole rationale in `openspec/changes/archive/*-add-uat-release-gate/design.md`
is: "v0.1.0's `vault --version` regression slipped past unit tests, clippy,
and release workflow." UAT-INSTALL-001 is the entry that re-runs the
`curl | bash | vault --version` path that originally exposed the bug in the
wild. First post-landing UAT run reproduces the bug — **the gate works as
designed**.

### Disposition

- Do NOT re-cut v0.1.0. CHANGELOG already promised v0.1.0 as-is.
- Tag v0.1.1 as soon as the in-flight keychain + SSE findings resolve
  (UAT-FIND-001, UAT-FIND-005).
- After v0.1.1 tag ships, re-run UAT-INSTALL-001 — expected **PASS**, since
  the latest-release endpoint now serves the binary built from commit
  `5990f12` or later.

### Regression guard

UAT-INSTALL-001 already exists. Strengthen the Pass criterion to also assert
that the tarball SHA256 matches the one checked into `install.sh` once signing
lands (roadmap: code-signing + notarization).

---

## Triage conventions

- **New UAT run FAILs** → add entry here first, then fix. An unregistered
  failure is a gap in the audit trail.
- **Severity inflation / deflation** allowed but traceable: add a dated note
  in the entry, don't silently rewrite.
- **Closing a finding** requires (a) the fix commit hash, (b) a regression
  guard (new UAT entry or an existing one that would have caught it). If no
  regression guard exists, the finding closes with status `Fixed (no guard)`
  and a roadmap item for the guard.
