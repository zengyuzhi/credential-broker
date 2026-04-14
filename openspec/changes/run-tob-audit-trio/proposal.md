## Why

credential-broker just shipped v0.1.0 with zero external audit. The binary handles secrets end-to-end: reads API keys out of the macOS Keychain, holds them in process memory, passes them through subprocess env vars or an HTTP proxy, and persists lease metadata in SQLite. Three audit skills now sit in the repo's plugin cache that each target a specific failure class we haven't validated against:

- **`zeroize-audit`** — do we wipe secret buffers from memory after use, and do our wipes survive compiler optimization? Rust's default `String`/`Vec<u8>` do not zero on drop.
- **`supply-chain-risk-auditor`** — what does our direct dependency tree look like on popularity/maintainer/CVE axes? `sqlx`, `security-framework`, `axum`, `askama`, `blake3`, `reqwest`, `clap` each carry different risk profiles; we've never scored them.
- **`sharp-edges`** — are we mis-using error-prone APIs? Async mutexes held across `.await`, `.unwrap()` in auth paths, `SystemTime` for token expiry, `security-framework` ACL surprises.

A one-shot baseline pass now (1) establishes a floor before v0.2.0 drift, (2) captures what "unaudited v0.1.0" actually looks like in writing, and (3) folds the trio into `docs/RELEASE.md`'s pre-tag checklist so every future release re-runs them.

## What Changes

- Run each of the three audit skills against the current workspace. Capture raw findings in `docs/audits/2026-04-14-tob-baseline/` — one file per skill.
- Consolidate findings into a single `docs/audits/2026-04-14-tob-baseline/SUMMARY.md` with severity-ranked items (CRITICAL / HIGH / MEDIUM / LOW / INFO) and one of three dispositions per item: **Fix now** (lands on `main` this change), **Triage** (filed to `docs/ROADMAP.md` with a milestone tag), **Accept** (documented rationale, no action).
- Apply CRITICAL + HIGH fixes on `main` in this change. MEDIUM and below go to ROADMAP unless trivial.
- Update `docs/RELEASE.md` release-readiness checklist: add a **Security audit pass** step that re-runs the trio and fails the checklist if any new CRITICAL/HIGH appears since the baseline.
- Update `CHANGELOG.md` `[Unreleased] → Security` with any fixes that land.
- Add a top-level `docs/audits/README.md` explaining the audit directory convention (dated baselines, re-run cadence, disposition taxonomy).

## Capabilities

### New Capabilities
- `security-audit-baseline`: The project's audit protocol — which skills run, what their outputs look like, where findings get stored, how dispositions are tracked, and when the protocol re-runs (pre-release + on demand).

### Modified Capabilities
- `release-process`: The release-readiness checklist in `docs/RELEASE.md` gains a **Security audit pass** step that invokes the `security-audit-baseline` protocol and compares findings against the most recent baseline.

## Impact

- **New directory**: `docs/audits/` containing the README and the first dated baseline (`2026-04-14-tob-baseline/`).
- **Modified files**: `docs/RELEASE.md` (checklist addition), `CHANGELOG.md` (Unreleased Security section), `docs/ROADMAP.md` (any triaged items).
- **Possible code changes**: fixes for CRITICAL/HIGH findings. Exact files unknown until findings are in; scope is bounded by the disposition rule above.
- **New capability spec**: `openspec/specs/security-audit-baseline/spec.md` (on sync after archive).
- **Delta spec**: `openspec/changes/run-tob-audit-trio/specs/release-process/spec.md` appending the checklist step.
- **No new runtime dependencies**: the three ToB skills live in the plugin cache; they are invoked, not vendored.
- **Affected crates**: potentially all, depending on findings. Most likely: `vault-secrets` (zeroize), `vault-policy` (token comparison), `vault-cli` (dependency surface), `vaultd` (auth paths).

## Security Implications

This change exists to improve security posture. The *process* of auditing itself has minor implications:

- **Finding disclosure**: raw audit output may mention exploit primitives. Before any finding graduates to `docs/audits/` it is reviewed for sensitive detail. In practice, for a pre-v0.1.0-adoption codebase with no users at risk, this is a low-stakes concern.
- **False-positive risk**: the skills are pattern-based and will produce FPs. Every item gets a human review gate before disposition; `fp-check` (ToB skill, optional install) is the canonical sweep if FPs overwhelm signal.
- **Scope creep**: the skills will surface items outside this session's fix budget. Hard cap: CRITICAL + HIGH fixes only on `main` in this change. Everything else is triaged to avoid a never-ending audit PR.

## Out of Scope

- **Dynamic analysis** (fuzzing, property-based testing of the lease state machine, runtime ACL probing). Separate change if we want it.
- **Third-party paid audit** (Trail of Bits engagement, Cure53, etc.). This is the free self-service pass.
- **`constant-time-analysis`** and other ToB skills we didn't install. The trio was chosen to stay focused; additions are a future change.
- **Linux/Windows audit coverage**. macOS-only until the port lands.
- **Automated CI integration of the trio**. The skills are interactive and Claude-invoked; running them headlessly in GitHub Actions is a follow-on story.
- **Fixing every MEDIUM or LOW finding.** Triage-to-roadmap is the explicit disposition for those unless trivial.
- **Rewriting `vault-secrets` around a different OS API.** Any finding that would require replacing `security-framework` with a different binding becomes a roadmap item, not a fix in this change.
