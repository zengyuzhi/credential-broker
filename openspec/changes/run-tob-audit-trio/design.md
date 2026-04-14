## Context

Three Trail of Bits audit skills are now installed at the repo level (`zeroize-audit`, `supply-chain-risk-auditor`, `sharp-edges`). Each targets a distinct, complementary failure class. None have ever been pointed at this codebase. The project just shipped v0.1.0 — the last moment where the delta between audited and unaudited state is small enough to fit in one change.

The skills are Claude-interactive: they spawn subagents, read source with semantic tools (some via LSP/MCP), and produce free-form reports. There is no CI-runnable headless mode today. That shapes the entire protocol below.

## Goals / Non-Goals

**Goals:**
- Produce a written, dated baseline of what the trio says about v0.1.0. Store it alongside code (`docs/audits/`) so future auditors can diff against it.
- Fix the findings that matter (CRITICAL + HIGH), triage the rest to `docs/ROADMAP.md`, and explicitly accept the ones we've decided not to fix.
- Make re-running the audit a required step on the release-readiness checklist so the baseline doesn't rot.
- Establish a reusable audit-directory convention that works for the next baseline (different skills, different date) without restructuring.

**Non-Goals:**
- Full audit coverage. Three skills ≠ a complete threat model.
- CI automation. The skills are interactive; don't pretend they aren't.
- Formal verification, fuzzing, or runtime analysis. Static-ish only.
- Fixing every finding. That's an infinite backlog; we cap at CRITICAL+HIGH.
- Paid external audit. Out of scope for a personal project.

## Decisions

### Decision 1: One directory per baseline, dated, per-skill file, one SUMMARY

Layout:
```
docs/audits/
├── README.md
└── 2026-04-14-tob-baseline/
    ├── SUMMARY.md
    ├── zeroize-audit.md
    ├── supply-chain-risk-auditor.md
    └── sharp-edges.md
```

**Why:** Dated directories preserve history without rewriting prior baselines. Per-skill files keep raw output navigable. A top-level `SUMMARY.md` per baseline is the human-facing artifact — the place dispositions live. `docs/audits/README.md` documents the convention so a stranger can read the structure.

**Alternatives considered:**
- *Single `docs/SECURITY_AUDIT.md` that gets overwritten* — rejected: destroys history.
- *GitHub Issues for each finding* — rejected for now: issue tracker is empty and low-ceremony markdown beats opening 20 issues for a personal project. Reconsider if we ever have collaborators.
- *Append-only log file* — rejected: hard to browse, conflates baselines.

### Decision 2: Severity scale — CRITICAL / HIGH / MEDIUM / LOW / INFO

Map the skills' output to this five-level scale in `SUMMARY.md`. Skills have their own conventions; we normalize.

**Rubric:**
- **CRITICAL**: remote code execution, credential theft, auth bypass, key leak to disk/log/network. Ship-stopper.
- **HIGH**: significant privilege escalation, non-constant-time comparison of secrets, missing zeroization of raw API keys, actively-maintained CVE in direct dep with no upgrade path.
- **MEDIUM**: defense-in-depth gap without active exploit path, sharp-edge misuse that's easy to hit but bounded in impact, dep with moderate risk signals.
- **LOW**: style-level crypto hygiene, docs/comment misleading, minor footgun.
- **INFO**: observation only; no action implied.

**Why:** Five levels is the common denominator across security tooling. Mapping first into our scale keeps disposition logic consistent regardless of which skill found it.

### Decision 3: Fix budget — CRITICAL + HIGH only, in this change

Everything else is disposed to ROADMAP or Accept. The change is a baseline + integration; it is not a "fix everything you find" marathon.

**Why:** Scope creep is the most common way audit PRs die. A clear cap lets findings land even when they surface surprises. If CRITICAL+HIGH alone is too much for one PR, split the fixes into follow-on changes but keep this one as the baseline-of-record.

**Alternatives considered:**
- *Fix everything CRITICAL through MEDIUM* — rejected: open-ended. Re-evaluate if the first run produces few findings.
- *Fix only CRITICAL; everything else roadmaps* — rejected: HIGHs like missing zeroize on API key buffers are the kind of thing an "unreleased" project should fix before the first user depends on a behavior.

### Decision 4: Disposition taxonomy — Fix now / Triage / Accept

Every item in `SUMMARY.md` gets exactly one disposition tag:
- **Fix now** — code change in this change; references the commit SHA
- **Triage** — moved to `docs/ROADMAP.md` (Near/Medium/Speculative) with a "from audit" tag
- **Accept** — documented rationale; no code change

**Why:** Three options are enough to move every finding to a terminal state. Two is too coarse ("fix or ignore"); four+ invites bikeshedding.

**Accept rationale templates:**
- "Low impact, macOS-only v0.1.0 scope" — personal-use posture absorbs it
- "Fix requires dependency replacement" — roadmaps instead
- "Skill false positive" — document the call chain that disproves it

### Decision 5: Release-checklist integration = comparative, not absolute

`docs/RELEASE.md` step 9 (new): "Security audit diff". Re-run the trio (or document why skipped). Compare output to the most recent `docs/audits/<date>/SUMMARY.md`. The release is **blocked** only if new CRITICAL or HIGH items appear relative to baseline. Delta-down (fewer findings) or same-set-lower-severity is fine.

**Why:** Absolute gates ("zero HIGHs forever") don't survive real use — the scanners churn, deps update, new patterns emerge. Comparative gates measure regression, which is what we actually care about.

**Alternatives considered:**
- *Absolute gate on zero CRITICAL/HIGH* — rejected: first false positive becomes a permanent release-blocker.
- *Pure informational, no gate* — rejected: then the step rots; nobody reads a checklist item that can't fail.

### Decision 6: Baseline supersession vs append

When a future release re-runs the trio, create a new dated directory (`docs/audits/2026-07-14-tob-pre-v0-2-0/` or similar) and update `docs/audits/README.md`'s "latest baseline" pointer. Old baselines stay on disk as history.

**Why:** Pointing at "the latest" is fragile if it's a symlink or convention. A single file pointer (the README) is the only thing release.md needs to resolve.

### Decision 7: Claude-interactive invocation, not CI

The skills run inside a Claude Code session. The checklist instructs the maintainer to invoke them via `Skill` tool or `/opsx`-adjacent slash commands. No GitHub Actions integration. A future change may automate, but not this one — pretending there's headless parity when there isn't just produces broken workflows.

**Why:** Honest tooling. If the skills move to a headless mode later, the checklist step changes in one line.

### Decision 8: If a skill errors or stalls, document and proceed

Don't block a baseline on a single flaky skill. If `sharp-edges` hangs or `supply-chain-risk-auditor` can't reach an external index, capture the failure in that skill's file (`zeroize-audit.md` or similar) and note the gap in `SUMMARY.md` under an "Incomplete scans" section. A partial baseline is better than no baseline.

**Why:** The alternative is never completing the pass. Document debt explicitly; it beats pretending the audit covered ground it didn't.

## Risks / Trade-offs

- **[High false-positive rate buries real signal]** → Mitigation: human review before dispositioning; optional `fp-check` skill install if noise dominates. Accept dispositions are explicitly allowed — pushing an FP to Accept with rationale is fine.
- **[Fix budget (CRITICAL+HIGH) is still too wide for one PR]** → Mitigation: split fixes into follow-on OpenSpec changes; keep the baseline-doc + ROADMAP updates in this change. The invariant is "baseline committed before next tag," not "all fixes in this commit."
- **[Skills depend on external services (crates.io, GitHub stats) that may change]** → Mitigation: each per-skill file timestamps the scan and records service version where available. Delta comparison focuses on what's in the baseline, not what's "currently reported."
- **[Baseline rots between releases]** → Mitigation: checklist step forces re-run. If the maintainer skips it, they document why in the release retrospective — so the debt is visible.
- **[Scope creep into "let's rewrite secret handling"]** → Mitigation: Accept disposition exists precisely for structural findings that deserve a standalone design. Never nest a refactor inside an audit change.
- **[Findings mention specific exploit primitives]** → Mitigation: raw per-skill files redact exploit code if any surfaces; `SUMMARY.md` describes the class of issue and fix, not the proof-of-concept.

## Migration Plan

**Local, reversible:**
1. Create `docs/audits/README.md` + `docs/audits/2026-04-14-tob-baseline/` empty structure.
2. Invoke each of the three skills (one at a time to keep output manageable). Save raw output to per-skill files.
3. Read all three, normalize into `SUMMARY.md` using the severity rubric + disposition tags.
4. For every CRITICAL/HIGH marked "Fix now": apply the fix, cite the commit SHA in `SUMMARY.md`, add a `CHANGELOG.md` `[Unreleased] → Security` bullet.
5. For every "Triage": copy to `docs/ROADMAP.md` with an "audit: <skill>" tag so we can find them later.
6. Update `docs/RELEASE.md` checklist step 9 + delta spec for `release-process`.
7. Run `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`. These must stay green after fixes.
8. Commit in logical chunks (per-fix commits are fine; baseline-docs in a final `docs: ...` commit).

**External effect:** none. No tag push, no public action. This change ends on `main` like any other docs+fix change.

**Rollback:** `git revert` the fix commits; `docs/audits/` history stays as a record. No external state to unwind.

## Open Questions

- **Should we also run `audit-context-building` before the trio?** Probably yes as a zero-cost context primer, but the output goes into the scanner's context, not into `docs/audits/`. Decide at apply time.
- **Disposition for findings that span multiple crates** (e.g., zeroize missing in *every* `String` that carries a secret)? Group into a single SUMMARY item with a per-crate checklist in the body. Reassess if the first run produces more than 5 such groups.
- **Should `Accept` findings roll into `CHANGELOG.md` "Known limitations"?** Leaning yes for user-visible accepts (e.g., "no constant-time eq on PIN because macOS already rate-limits via burn counter"); internal accepts stay in `SUMMARY.md` only. Call it at apply time.
