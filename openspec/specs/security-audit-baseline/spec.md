# Spec: Security Audit Baseline

## Purpose

Defines the requirements for conducting, storing, and maintaining security audit baselines for the credential broker. A baseline is a dated, structured record of findings produced by running the audit skill trio (zeroize-audit, supply-chain-risk-auditor, sharp-edges) against the codebase. Baselines gate releases and provide a traceable history of the project's security posture.

## Requirements

### Requirement: Audit baselines live under `docs/audits/<YYYY-MM-DD>-<slug>/`

The project SHALL store every security audit baseline as a dated subdirectory under `docs/audits/`. Each baseline subdirectory MUST contain at minimum: one per-skill raw-output file and one `SUMMARY.md` consolidating findings. A single top-level `docs/audits/README.md` MUST document the convention and name the most recent baseline.

#### Scenario: New baseline creates a dated directory

- **GIVEN** a maintainer begins a new audit pass on `YYYY-MM-DD`
- **WHEN** they run the audit protocol
- **THEN** they create `docs/audits/<YYYY-MM-DD>-<slug>/` where `<slug>` identifies the pass (e.g., `tob-baseline`, `pre-v0-2-0`)
- **AND** they add one file per skill invoked (e.g., `zeroize-audit.md`, `supply-chain-risk-auditor.md`, `sharp-edges.md`)
- **AND** they add a `SUMMARY.md` that consolidates findings across skills

#### Scenario: README points to the latest baseline

- **WHEN** a release-checklist step needs to compare against "the current baseline"
- **THEN** `docs/audits/README.md` names one dated subdirectory as "latest"
- **AND** the pointer is updated in the same commit that lands a new baseline
- **AND** older baselines remain on disk as history — they are not deleted

### Requirement: Each finding carries a severity and a disposition

The SUMMARY.md for a baseline SHALL classify every finding with (1) a severity on a five-level scale — **CRITICAL / HIGH / MEDIUM / LOW / INFO** — and (2) a disposition tag — **Fix now / Triage / Accept**. Every finding MUST end in exactly one terminal disposition; no "pending" or "TBD" items are allowed in a committed SUMMARY.

#### Scenario: Severity rubric is explicit

- **WHEN** a reader opens `SUMMARY.md`
- **THEN** a severity rubric block defines CRITICAL / HIGH / MEDIUM / LOW / INFO in one or two lines each
- **AND** every item references one of those five labels verbatim (not synonyms like "P0" or "Severe")

#### Scenario: Fix-now items cite the landing commit

- **GIVEN** a finding is marked `Disposition: Fix now`
- **WHEN** the fix is committed
- **THEN** the SUMMARY.md item is updated in the same change to include the commit short-SHA (or SHA list if multi-commit) that resolved it
- **AND** the fix lands on `main` before the change is archived

#### Scenario: Triaged items move to `docs/ROADMAP.md`

- **GIVEN** a finding is marked `Disposition: Triage`
- **WHEN** the SUMMARY.md is finalized
- **THEN** a corresponding bullet exists in `docs/ROADMAP.md` (under Near-term, Medium-term, or Speculative as appropriate)
- **AND** the roadmap bullet tags the finding's origin as `(audit: <skill-name> YYYY-MM-DD)` so the source is traceable

#### Scenario: Accepted items document rationale

- **GIVEN** a finding is marked `Disposition: Accept`
- **WHEN** the SUMMARY.md entry is written
- **THEN** the entry includes a "Rationale" paragraph explaining why no action is taken (e.g., "false positive — <call chain>", "mitigated by <control>", "scope-out per design.md decision N")
- **AND** the paragraph is specific enough that a stranger can audit the audit

### Requirement: CRITICAL and HIGH findings block archive until dispositioned

The change that introduces a baseline SHALL NOT be archived while any CRITICAL or HIGH finding in that baseline's SUMMARY.md has a `Fix now` disposition without a landing commit SHA. Accept and Triage dispositions are permitted for any severity, provided their rationale/roadmap bullet exists.

#### Scenario: Unresolved Fix-now CRITICAL blocks archive

- **GIVEN** SUMMARY.md lists one CRITICAL finding with `Disposition: Fix now` and no commit SHA
- **WHEN** `/opsx:archive` is invoked on the change
- **THEN** the maintainer either (a) lands the fix and adds the SHA, (b) re-disposes to Triage or Accept with appropriate docs updates, or (c) blocks archive
- **AND** the `SUMMARY.md` never enters main spec with unresolved Fix-now items

#### Scenario: Accepted CRITICAL is allowed if rationale is explicit

- **GIVEN** a CRITICAL finding is a confirmed false positive
- **WHEN** it is disposed to `Accept` with an explicit Rationale paragraph proving the FP
- **THEN** the change is archivable without a code fix
- **AND** the FP call-chain is preserved in SUMMARY.md for future maintainers who see the same scanner output

### Requirement: Skills invoked are documented and reproducible

The per-skill files in a baseline SHALL record (1) the skill identifier as it appears in the Claude Code plugin system, (2) the invocation date, (3) the skill version if available, and (4) any inputs beyond "the current workspace" (e.g., excluded paths, focus directories).

#### Scenario: Skill file includes provenance header

- **WHEN** a per-skill file is created
- **THEN** its first section reads: skill identifier (e.g., `zeroize-audit@trailofbits/skills`), invocation date (ISO 8601), scope (files/dirs scanned or "workspace root"), and any flags or config
- **AND** the rest of the file is the raw or lightly-edited scanner output

#### Scenario: Partial scans are marked

- **GIVEN** a skill errors out, stalls, or is aborted before producing output
- **WHEN** the baseline is finalized
- **THEN** that skill's file still exists and documents the failure mode
- **AND** `SUMMARY.md` has an "Incomplete scans" section naming which skills did not complete and why

### Requirement: Baseline is re-run as part of the release checklist

The `docs/RELEASE.md` release-readiness checklist SHALL include a Security audit step that invokes the baseline protocol and blocks the tag push if any *new* CRITICAL or HIGH finding appears relative to the most-recent archived baseline.

#### Scenario: Re-run produces a new dated baseline before a release

- **GIVEN** a maintainer is cutting version `vX.Y.Z`
- **WHEN** they reach the Security audit step in `docs/RELEASE.md`
- **THEN** they either create a new `docs/audits/<YYYY-MM-DD>-<slug>/` baseline directory or document in the release retrospective why the step was skipped (e.g., "docs-only release, no code delta")
- **AND** any new CRITICAL or HIGH relative to the prior baseline gate the tag push

#### Scenario: Equal-or-lower-risk delta passes the gate

- **GIVEN** the new baseline contains the same findings as prior, or fewer, or the same count at lower severities
- **WHEN** the Security audit step is evaluated
- **THEN** the checklist step passes
- **AND** the release proceeds
