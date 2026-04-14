## Context

After shipping v0.1.0, two regressions proved that the existing release gate (unit tests + clippy + fmt + audit) had blind spots on the user-facing surface:

1. `vault --version` advertised in README, CHANGELOG, and the release body was broken — clap `#[command(version)]` was never wired. All 73 unit tests passed; the release workflow went green. A 30-second `cargo run -p vault-cli -- --version` check would have caught it.
2. The install-script end-to-end smoke test (`curl | bash | vault --version`) *did* catch it, post-tag. But that was ad-hoc, not on any checklist.

At the same time, three other forces made UAT structure overdue:
- **Surface growth:** 14 canonical capabilities in two weeks. The "I'll eyeball it" approach stops working past ~10.
- **AI-agent execution:** multiple sessions in this repo have been asked to "run the smoke tests" and each has invented its own structure. Without a contract, Claude re-invents the tag taxonomy, pass criteria, and reporting format every time.
- **No parallel to the audit gate:** `security-audit-baseline` is formalized (dated-directory convention, severity rubric, disposition taxonomy, comparative gate). Functional coverage has nothing equivalent.

The `security-audit-baseline` pattern is the template: a doc (`docs/audits/README.md`), per-run artifacts in dated subdirectories, a formal capability spec, and a wired release-process step. This change applies the same pattern to functional UAT.

## Goals / Non-Goals

**Goals:**
- One source of truth (`docs/UAT.md`) that serves humans, a future shell runner, and AI agents.
- Machine-parseable entry format — grep-friendly tags, regex pass-criteria — that a naive bash script can dispatch from.
- Explicit AI-agent guideline inside the doc so any session can pick it up without re-scoping.
- Release-process integration that *blocks* a tag on golden-path failure, not just warns.
- A first-run artifact (`docs/uat-runs/2026-04-14-v0.1.1-pre.md`) that doubles as the template for future runs.
- Cover the 14 canonical capabilities and 4 golden-path flows — ~25 entries, not 100.

**Non-Goals:**
- Writing the shell runner (`scripts/uat.sh`). The doc alone is sufficient; the runner is optimization.
- Autogeneration from OpenAPI/JSON schemas — manual authoring is fine at 25 entries.
- CI integration. Once the runner exists, a `uat-pr` workflow becomes obvious.
- Cross-platform UAT — Linux/Windows entries wait on those ports existing.
- Paid-provider assertion beyond response shape — belongs in integration tests, not UAT.
- Performance/load testing — different gate, different doc.
- Retroactively backfilling UAT entries for archived changes.

## Decisions

### Decision 1 — Single file vs split

**Choice:** single `docs/UAT.md`.

**Why:** three consumers (human, script, AI) all grep the same file. A split (one file per capability, or separate "ai-runner" doc) creates drift: the doc and the runner fall out of sync, the AI section references stale tag names. One file, one grep, one source of truth. If the file grows past ~500 lines, split later.

**Alternatives considered:** per-capability files under `docs/uat/`. Rejected — bash runner would need a glob + merge; AI agent would need to read N files; humans would jump between tabs.

### Decision 2 — Entry format: structured bullets under `####` heading

**Choice:** each UAT entry is an `#### UAT-<area>-NNN` heading with a fixed 5-field bullet block:
```
#### UAT-CLI-001 — <short description>
- **Type:** `[AUTO:ANY]`
- **Cap:** cli-help-text
- **Cmd:** `cargo run -p vault-cli -- --version`
- **Pass:** stdout matches `^vault \d+\.\d+\.\d+$`
- **AI-safe:** yes
```

**Why:** markdown `####` is grep-friendly (`grep '^#### UAT-' docs/UAT.md` lists all IDs), bold-prefix fields parse cleanly with `awk '/\*\*Cmd:\*\*/'`, and the block is human-readable. A YAML front-matter per entry would be more machine-friendly but heavier for human authors.

**Alternatives considered:**
- YAML block per entry — clean parsing, poor readability.
- Inline table row — dense, but multi-line `Pass` regex or `Steps` don't fit.
- HTML comments as metadata — invisible to the reader.

### Decision 3 — Four type tags, not two

**Choice:** `[AUTO:ANY]`, `[AUTO:CI]`, `[MANUAL:SHELL]`, `[MANUAL:USER]`.

**Why:** AI agents and shell scripts have different capabilities. An AI tool call can run `vault --version` and match stdout — that's `[AUTO:ANY]`. An AI tool call *cannot* run `vault serve --background` + poll `/health` + `vault serve stop` as a cohesive unit, because each tool call is independent and the PID file cleanup might straddle tool-call boundaries — that's `[AUTO:CI]`, fine for a shell script but flagged unsafe for AI. Keychain prompts and browser interactions split further: a keychain prompt needs the *user's keyboard* (`[MANUAL:SHELL]`), a dashboard SSE live-update needs the *user's eyes* (`[MANUAL:USER]`). Conflating these two has bit us before — an AI agent claimed "PASS" on a manual item because no automated check failed.

**Alternatives considered:**
- Two tags (`auto` / `manual`) — loses the AI-vs-CI distinction; AI agents will try to run `[AUTO:CI]` entries and get stuck.
- Three tags (`auto`, `manual`, `interactive`) — doesn't distinguish keyboard-needed from eyes-needed; both collapse to "interactive".

### Decision 4 — Pass-criteria: regex for AUTO, free-text for MANUAL

**Choice:** `[AUTO:*]` entries MUST have a regex in the `Pass:` field. `[MANUAL:*]` entries have free-text pass criteria.

**Why:** regex is deterministic — a shell script and an AI agent both produce the same PASS/FAIL for the same stdout. Free-text for MANUAL acknowledges that human judgment is doing the checking anyway ("dashboard updates within 4 seconds") and no regex could codify it without over-specifying.

**Edge case:** some `[AUTO:*]` entries legitimately match on exit code, not stdout (e.g., `cargo clippy -- -D warnings`). Format allows `Pass: exit 0` as a degenerate regex.

### Decision 5 — Run-log storage: `docs/uat-runs/<YYYY-MM-DD>-<version>.md`

**Choice:** separate dated files, not append to `docs/UAT.md` or `docs/RELEASE.md`.

**Why:** mirrors the `docs/audits/<YYYY-MM-DD>-<slug>/` convention. Append-only to UAT.md would create a mega-document; appending to RELEASE.md's retrospective section works for 2-3 releases then becomes unreadable. Dated directory means `ls docs/uat-runs/` is the release history, same muscle memory as `ls docs/audits/`.

**File format:**
```
---
version: v0.1.1-pre
date: 2026-04-14
runner: claude-opus-4-6 (1M context)
commit_sha: <sha-at-run-time>
---

## Summary
- Total: 24 | Pass: 22 | Fail: 1 | Skip: 1
- Golden path: 4/4 ✓
- Gate: PASS

## Results

| UAT-ID | Type | Result | Evidence |
|--------|------|--------|----------|
| UAT-CLI-001 | [AUTO:ANY] | PASS | `vault 0.1.0` |
| UAT-DASH-003 | [MANUAL:USER] | PASS (user confirmed) | — |
| UAT-PROXY-OAI-001 | [MANUAL:USER] | SKIP | $UAT_ALLOW_PAID not set |
...

## Failures
- UAT-XXX-007: <details + next action>
```

### Decision 6 — Release-process integration: comparative gate, step 4.5

**Choice:** new step between step 4 (audit pass) and step 5 (CHANGELOG). Gate logic:
- **Golden-path entries** (`UAT-CLI-001..004`): 4/4 MUST pass. Any golden-path failure blocks the tag.
- **Other `[AUTO:ANY]` entries:** ≥95% pass rate. Allows for a single flaky entry per run.
- **`[MANUAL:USER]` entries:** ≥80% pass rate. Acknowledges that some visual checks degrade over time without immediate code regression.
- **`[AUTO:CI]` and `[MANUAL:SHELL]`:** informational only at the release gate — they're too expensive or too fragile to block on.
- **Skip allowed:** any entry can be `SKIP`'d with documented reason (e.g., `$UAT_ALLOW_PAID not set`, `Keychain-locked, physical access required`). Skips don't count toward the denominator.

**Why:** matches the comparative / risk-calibrated philosophy from the audit gate. Absolute-100% would make UAT a blocker for every flaky entry; no gate at all would let the next `--version` regression slip through.

**Docs-only release skip:** same carve-out as the audit gate. If `git diff <last-tag>` touches only `*.md`, UAT can be skipped with the phrase `UAT skipped: docs-only diff since <prior-run-path>` in the retrospective.

### Decision 7 — AI-agent guideline: in-file section, not separate skill file

**Choice:** the guideline lives inside `docs/UAT.md` as a dedicated `## Running as an AI agent` section.

**Why:** the user already made this call in the pre-propose discussion. A separate skill file at `~/.claude/skills/uat-run.md` would need to be installed per-machine; an in-file section travels with the repo. Trade-off: the section adds ~50 lines to the doc; worth it for self-containment.

**What the section says:** step-by-step runbook for an AI reading the file, tool-call conventions, forbidden actions (claim PASS without running, skip MANUAL silently, invent UAT-IDs, run paid entries without explicit consent).

### Decision 8 — First-pass entry count: ~25, not 50+

**Choice:** cover the 14 canonical capabilities with 1–2 entries each + the 4 golden-path CLI flows = ~25 entries.

**Why:** a 25-entry doc is walkable in 30–45 minutes. 50+ entries dilutes the signal and discourages running the full sweep. Once we see which entries consistently catch bugs and which never flip, we can grow (or prune) from evidence.

**Structural coverage target:**
- 4 golden-path (`UAT-CLI-001..004`)
- 3 CLI surface (`--version`, `--help` trees, `vault stats --json`)
- 4 dashboard (`UAT-DASH-*`) — login, CSRF rejection, SSE live-update, page render smoke
- 3 serve lifecycle (`UAT-SERVE-*`)
- 3 proxy (`UAT-PROXY-*`) — OpenAI, Anthropic, TwitterAPI, all paid-gated
- 3 migration / install (`UAT-INSTALL-*`, `UAT-MIGRATE-*`)
- 3 security-regression (`UAT-SEC-*`) — PIN burn, rate-limit, CSRF
- 2 error-surface (`UAT-ERR-*`) — bad profile, expired lease

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| UAT doc drifts from reality as capabilities evolve | Archive-gate rule: every change that modifies a UAT-covered capability MUST update the matching UAT entry in the same change. Enforced by code review, not tooling (yet) |
| `[MANUAL:USER]` entries become "always pass because nobody runs them carefully" | Run-log requires evidence field — if evidence is blank, the entry is a SKIP, not a PASS |
| Paid-provider entries cost real money | `$UAT_ALLOW_PAID=1` env gate + AI-agent forbidden-action list. Default: SKIP |
| AI agent hallucinates a PASS on an AUTO entry | Run-log evidence field stores first 500 chars of actual stdout; reviewer can spot-check. Also, regex-on-stdout is auditable after the fact |
| 25 entries is too few to catch regressions | First-run retrospective identifies gaps; v0.1.2 adds more based on evidence. Starting small is cheaper than starting big and pruning |
| 25 entries is too many for every release | `[AUTO:ANY]` runs fast (~30 seconds); full sweep with manual runs in ~15 minutes. Acceptable for a release cadence of "weeks, not days" |
| Structured format locks us in early | Format is markdown — easy to migrate. Actual lock-in is the tag taxonomy; 4 tags is small enough to refactor |
| AI-agent guideline drifts from Claude's actual tool surface | Section references tool categories, not specific tool names. If `Bash` gets renamed, the guideline still makes sense. Review section at each audit baseline |

## Migration Plan

1. Create `docs/UAT.md` with: preamble + tag taxonomy + AI-agent guideline section + ~25 entries structured by capability + run-log template appendix.
2. Create `docs/uat-runs/` directory (empty at this stage — will hold actual run files).
3. Run today's first pass → produce `docs/uat-runs/2026-04-14-v0.1.1-pre.md`.
4. Update `docs/RELEASE.md` — insert step 4.5 "UAT pass" with comparative-gate logic.
5. Update `CHANGELOG.md [Unreleased]` to cite the new UAT gate.
6. Commit as single change: `docs(uat): add release-gate UAT checklist and first-pass run`.
7. Push to `main`.

**Rollback:** `git revert` the single commit. UAT is doc-only, no binary or DB impact.

## Open Questions

- **Should `[AUTO:CI]` entries be renamed `[AUTO:SHELL]` for consistency with `[MANUAL:SHELL]`?** Proposal uses `[AUTO:CI]` because these entries *will* eventually run in CI; but they're runnable from any shell today. Either name works. Defer to implementation — rename costs one `sed`.
- **How should the run-log represent a partial run (e.g., 3 of 4 golden-path completed, user abandoned)?** Proposal: front-matter `status: partial`, table shows completed rows only, summary shows "incomplete — do not use for gate decisions." Needs wording in the spec.
- **Does the AI-agent guideline need to enumerate specific forbidden slash-commands (e.g., never invoke `/opsx:archive` during a UAT run)?** Leaning yes — easier to be explicit.
- **Should paid-provider entries have a cost estimate field** (e.g., `EstCost: $0.002`)? Would help users decide whether to flip `$UAT_ALLOW_PAID`. Low-effort, high-value. Add it.
