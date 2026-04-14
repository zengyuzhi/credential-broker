## ADDED Requirements

### Requirement: Release checklist includes a security-audit-baseline diff step

The `docs/RELEASE.md` release-readiness checklist SHALL include a numbered step titled **Security audit pass** that invokes the `security-audit-baseline` protocol and compares results against the most-recently-archived baseline. The step MUST appear after the test / clippy / fmt gates (so it does not run on broken code) and before the version-bump step (so any code change it motivates happens before the tag).

#### Scenario: Security audit pass runs the trio and produces a dated baseline

- **GIVEN** a maintainer has reached the Security audit step in `docs/RELEASE.md`
- **WHEN** they execute the step
- **THEN** they create a new `docs/audits/<YYYY-MM-DD>-<slug>/` directory containing one per-skill file plus `SUMMARY.md`, following the `security-audit-baseline` capability conventions
- **AND** the `docs/audits/README.md` "latest baseline" pointer is updated in the same commit that lands the baseline

#### Scenario: New CRITICAL or HIGH relative to prior baseline blocks the tag

- **GIVEN** the newly-produced SUMMARY.md contains any CRITICAL or HIGH finding that did not exist in the prior baseline
- **WHEN** the maintainer evaluates whether to proceed to the tag push step
- **THEN** the checklist instructs them to stop, disposition the new finding (Fix now / Triage / Accept per the `security-audit-baseline` rules), and restart the checklist from the top
- **AND** no tag is pushed until the disposition closes the gap

#### Scenario: Equal-or-lower-risk delta passes the gate

- **GIVEN** the new baseline has the same findings as prior, or fewer, or the same count at lower severities
- **WHEN** the maintainer evaluates the step
- **THEN** the step passes and the checklist continues to the version bump
- **AND** the new baseline is committed alongside the release docs bump

#### Scenario: Step is documentably skippable only when no code changed

- **GIVEN** the release is a pure documentation or configuration change with zero Rust source diff since the last baseline
- **WHEN** the maintainer reaches the Security audit step
- **THEN** they are permitted to skip re-running the trio and instead reference the prior baseline in the release retrospective section of `docs/RELEASE.md`
- **AND** the retrospective entry states explicitly "Security audit skipped: no code delta since `<prior-baseline-path>`"

#### Scenario: Audit step failure does not silently pass

- **GIVEN** one or more audit skills errors out, stalls, or cannot be invoked
- **WHEN** the maintainer reaches the Security audit step
- **THEN** the checklist requires that partial results are still committed to the dated baseline directory per the `security-audit-baseline` "Incomplete scans" scenario
- **AND** the gate decision uses only the findings that *did* complete, with the gap explicitly flagged in `SUMMARY.md`
- **AND** if no skill completed at all, the step fails and the tag push is blocked
