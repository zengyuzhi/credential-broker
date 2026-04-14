## ADDED Requirements

### Requirement: Release checklist includes a UAT pass step

The release procedure in `docs/RELEASE.md` SHALL include a numbered step (ordinally between the existing "Security audit pass" step and the existing "CHANGELOG current" step) that runs the UAT release gate defined by the `uat-release-gate` capability. The step SHALL produce exactly one run-log at `docs/uat-runs/<YYYY-MM-DD>-<version>.md` and SHALL block tag push on `Gate: FAIL`. The step SHALL be skippable only for docs-only releases (`git diff <last-tag>..HEAD` touches no files outside `*.md`, `docs/`, `openspec/`), in which case the skip SHALL be recorded in the release retrospective using the phrase `UAT skipped: docs-only diff since <prior-run-path>`.

#### Scenario: UAT step runs and gates the release

- **WHEN** a maintainer walks the release checklist for version `X.Y.Z`
- **THEN** they execute the UAT pass step after the security audit step and before rotating CHANGELOG
- **AND** a new file `docs/uat-runs/<date>-vX.Y.Z.md` exists before they proceed to the version bump

#### Scenario: Gate FAIL blocks tag push

- **WHEN** the UAT run-log reports `Gate: FAIL` (golden-path failure or threshold miss)
- **THEN** the maintainer does not proceed to `git tag -a vX.Y.Z`
- **AND** they either fix the underlying defect and re-run, OR update the UAT entry's spec via a separate change before re-running

#### Scenario: Docs-only release skips UAT with documented phrasing

- **WHEN** `git diff <last-tag>..HEAD` shows no changes outside documentation paths
- **THEN** the UAT step may be skipped
- **AND** `docs/RELEASE.md`'s Retrospective section for that release contains the exact phrase `UAT skipped: docs-only diff since docs/uat-runs/<prior-path>.md`

#### Scenario: Partial UAT run (runner abandoned mid-pass)

- **WHEN** a run-log's front-matter carries `status: partial`
- **THEN** the release gate SHALL treat the run as `Gate: FAIL`
- **AND** the maintainer either completes the run or documents the reason the partial run is acceptable via a follow-up spec update

#### Scenario: AI-agent UAT run satisfies the gate

- **WHEN** the UAT step is executed by an AI agent producing `docs/uat-runs/<date>-vX.Y.Z-ai.md`
- **THEN** the file is treated identically to a human-produced run-log for gate purposes
- **AND** the AI-produced run-log is never a substitute for the `[MANUAL:*]` entries it flagged as `SKIP` — those still require a human follow-up run before tag push

#### Scenario: Retrospective cross-references the run-log

- **WHEN** the release is completed
- **THEN** the Retrospective section for that version cites the run-log path
- **AND** the CHANGELOG entry for that version carries at least one bullet crediting the UAT gate under `### Quality` or equivalent subsection
