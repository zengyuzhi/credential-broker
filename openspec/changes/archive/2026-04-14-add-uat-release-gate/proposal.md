## Why

v0.1.0 shipped with `cargo test` (unit correctness) and a security-audit gate (defensive posture), but nothing that verifies the *user-facing feature surface* actually works end-to-end before a tag goes out. The `vault --version` regression in v0.1.0 is the proof: 73/73 unit tests passed, clippy + fmt clean, release workflow green — and the flag advertised in README, CHANGELOG, and the release body was still broken because clap was never wired up. Nothing on the checklist would catch that class of bug.

Two trends make a UAT doc worth formalizing *now* rather than after more regressions:
1. This project grew from 1 to 14 canonical capabilities in two weeks — the surface is now too wide to eyeball before every release.
2. AI agents (Claude, Codex, Gemini) are increasingly running this workflow. Without a machine-readable UAT contract, each session re-invents which flows to test, how to tag them, and what counts as pass. That's churn we don't need.

This change formalizes UAT as a structured, re-runnable, AI-readable release gate — mirroring the `security-audit-baseline` pattern but for functional coverage rather than defensive coverage.

## What Changes

- **New `docs/UAT.md`** — single source of truth for UAT entries. Per-capability sections + persona-journey sections + run-log template + an in-file "Running as an AI agent" guideline.
- **UAT ID scheme** — entries numbered `UAT-<area>-NNN` (e.g. `UAT-CLI-001`, `UAT-DASH-004`). Areas mirror canonical capabilities + a CLI bucket.
- **Four type tags** driving runner dispatch:
  - `[AUTO:ANY]` — safe for AI, script, or human. Deterministic command with regex-matched stdout.
  - `[AUTO:CI]` — safe for shell/CI but not AI (spawns background servers, writes to keychain, requires shell state an AI tool call can't provide).
  - `[MANUAL:SHELL]` — requires the human's keyboard (keychain GUI prompt, `rpassword` stdin, browser).
  - `[MANUAL:USER]` — requires the human's eyes (visual-diff dashboard, SSE live-update, quarantine xattr behavior).
- **Pass-criteria format** — regex on stdout for `[AUTO:*]`, free-text step-by-step for `[MANUAL:*]`.
- **Run-log artifacts** — one file per run at `docs/uat-runs/<YYYY-MM-DD>-<version>.md`, front-matter + PASS/FAIL table per UAT-ID + evidence snippets. Cross-linked from CHANGELOG and the release retrospective.
- **Release-process integration** — `docs/RELEASE.md` gains step 4.5 "UAT pass" between the audit pass (step 4) and CHANGELOG rotation (step 5). Comparative gate: ≥95% pass on `[AUTO:ANY]` AND 100% pass on golden-path entries (`UAT-CLI-001..004` covering `credential add → profile bind → vault run → vault stats`).
- **First-pass coverage** — ~25 UAT entries covering the 14 canonical capabilities plus the 4 golden-path CLI flows. Speculative/Linux/multi-user items excluded until those capabilities exist.
- **Today's first run becomes the first run-log** — `docs/uat-runs/2026-04-14-v0.1.1-pre.md`, attached to the change as the baseline retrospective.

## Capabilities

### New Capabilities

- `uat-release-gate`: Formalizes UAT doc schema, tag taxonomy, pass/fail semantics, run-log storage, AI-agent runner contract, and the comparative gate integration with `release-process`.

### Modified Capabilities

- `release-process`: Adds a new "UAT pass" step (step 4.5) to the release-readiness checklist, between the existing "Security audit pass" step and the "CHANGELOG current" step. Comparative gate logic mirrors the audit pattern.

## Impact

- **New docs**: `docs/UAT.md`, `docs/uat-runs/` directory + first run-log file.
- **Existing docs modified**: `docs/RELEASE.md` (new step 4.5 + retrospective entry template), `CHANGELOG.md` `[Unreleased]` (add a "Quality" or extend "Security" to cite the gate).
- **No Rust code changes** — this is process + documentation, no crate touches.
- **No new dependencies.**
- **Future follow-on** (out of scope here): `scripts/uat.sh` bash+awk runner that extracts `Cmd:` lines by type tag and executes the `[AUTO:*]` set unattended.

## Security Implications

UAT entries that touch secrets paths (`[MANUAL:SHELL]` keychain-prompt tests, `[MANUAL:USER]` dashboard PIN-login tests) MUST NOT record the raw secret in the run-log. Pass/fail evidence is limited to metadata: exit code, the first 500 chars of stdout *after* a scrub pass, session cookie presence (but not value), provider response shape (but not content).

Paid-provider UAT entries (`UAT-PROXY-OAI-001`, `UAT-PROXY-ANT-001`) stay behind a `$UAT_ALLOW_PAID=1` env gate so an unattended runner cannot burn money against the author's API account. AI-agent guideline section explicitly forbids setting this env var without explicit user consent in-session.

Threat model this does *not* defend against: a compromised CI runner with `$UAT_ALLOW_PAID=1` set — same risk surface as our existing `OPENAI_API_KEY` handling, tracked under the existing audit baseline.

## Out of Scope

- **`scripts/uat.sh` runner** — defer to a follow-up change `add-uat-shell-runner`. The doc alone is sufficient for manual human + AI execution; the script is optimization.
- **CI integration** — no GitHub Actions workflow change this pass. Once `scripts/uat.sh` exists, a `uat-pr` workflow that runs the `[AUTO:ANY]` subset on every PR becomes a natural follow-up.
- **Linux / Windows UAT entries** — only macOS flows this pass; cross-platform coverage waits on the Linux port per `docs/ROADMAP.md` Medium-term.
- **Paid-provider assertion beyond response-shape** — entry `UAT-PROXY-OAI-001` checks that OpenAI returns a 200 with a `"object": "list"` shape. It does *not* assert model availability, pricing, or rate-limit behavior — those belong in provider-adapter integration tests, not UAT.
- **Updating the archived `add-web-dashboard` or `ship-v0-1-0` CHANGELOG retros** — historical entries stay frozen.
- **Autogenerated UAT entries from OpenAPI / JSON schemas** — manual authoring is fine at 25-entry scale. Revisit if the surface grows past ~100 entries.
