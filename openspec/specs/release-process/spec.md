# Spec: Release Process

## Purpose

Defines the requirements for maintaining changelogs, release documentation, roadmap, and the mechanics of cutting and publishing versioned releases of the credential broker.

## Requirements

### Requirement: CHANGELOG.md tracks every released version

The project SHALL maintain a `CHANGELOG.md` at the repository root in Keep-a-Changelog 1.1.0 format. Every published Git tag of the form `vMAJOR.MINOR.PATCH` MUST have a corresponding `## [MAJOR.MINOR.PATCH] - YYYY-MM-DD` section listing changes grouped under the standard subsections (`### Added`, `### Changed`, `### Deprecated`, `### Removed`, `### Fixed`, `### Security`). An `## [Unreleased]` section at the top SHALL collect changes destined for the next tag.

#### Scenario: Initial v0.1.0 entry lists shipped features

- **GIVEN** `CHANGELOG.md` has just been created
- **WHEN** the reader opens the `## [0.1.0]` section
- **THEN** it enumerates credential management (add/list/enable/disable/remove), profiles with provider bindings, `vault run` env injection, HTTP proxy with lease-token auth, web dashboard with PIN auth and SSE live updates, `vault serve` foreground/background/status/stop, `vault ui` auto-start, `vault stats` with `--json` and `--provider` flags, GitHub Actions CI, release workflow producing macOS tarballs, and the `install.sh` one-liner installer
- **AND** a "Known limitations" note calls out macOS-only, no code signing (users must strip quarantine attribute), and personal-scale polish

#### Scenario: Unreleased section captures post-v0.1.0 changes

- **WHEN** a developer merges a user-visible change after v0.1.0 is tagged
- **THEN** they add a bullet under `## [Unreleased]` in the appropriate subsection before the PR merges
- **AND** the bullet describes the change in user-facing language, not implementation detail

### Requirement: Release procedure is documented in docs/RELEASE.md

The project SHALL maintain `docs/RELEASE.md` describing the end-to-end procedure for cutting a release. The document MUST cover: (1) where versions live and how to bump them, (2) the release-readiness checklist, (3) the tag-and-push command, (4) how to verify the published release, and (5) how to roll back a bad tag.

#### Scenario: Checklist gates every release

- **GIVEN** a maintainer intends to cut version `vX.Y.Z`
- **WHEN** they open `docs/RELEASE.md`
- **THEN** they find a numbered checklist with at minimum: `cargo test --workspace` green, `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo fmt --all -- --check` clean, CHANGELOG `[Unreleased]` moved to `[X.Y.Z] - <today>` with accurate contents, `vault --help` output reviewed for drift, version bumped in `crates/vault-cli/Cargo.toml`, and a clean `git status`
- **AND** each item has a copy-paste command where applicable

#### Scenario: Tag push triggers the release workflow

- **GIVEN** the checklist passes and changes are committed to `main`
- **WHEN** the maintainer runs `git tag -a vX.Y.Z -m "Release X.Y.Z" && git push origin vX.Y.Z`
- **THEN** `docs/RELEASE.md` states that `.github/workflows/release.yml` fires, builds matrix targets `aarch64-apple-darwin` and `x86_64-apple-darwin`, and uploads `vault-<target>.tar.gz` assets to a new GitHub Release
- **AND** the doc lists the verification step: `curl -fsSL https://raw.githubusercontent.com/zengyuzhi/credential-broker/main/install.sh | bash` pulls the new version and `vault --version` reports `vault-cli X.Y.Z`

#### Scenario: Bad tag is rolled back cleanly

- **GIVEN** a tag was pushed but the release workflow failed or produced a broken artifact
- **WHEN** the maintainer consults `docs/RELEASE.md`'s rollback section
- **THEN** the documented steps are: delete the GitHub Release via `gh release delete vX.Y.Z`, delete the remote tag via `git push origin :refs/tags/vX.Y.Z`, delete the local tag via `git tag -d vX.Y.Z`, fix the underlying issue, increment the patch version, and re-tag
- **AND** the doc warns that once a tag's binary has been downloaded by anyone, a new patch version is preferable to reusing the tag

### Requirement: Roadmap document exists and is referenced

The project SHALL maintain `docs/ROADMAP.md` listing post-v0.1.0 candidate work, grouped loosely by milestone (e.g., "Near-term", "Medium-term", "Speculative"). The document is explicitly a reference, not a commitment. `README.md` MUST link to `docs/ROADMAP.md` so users can find it without cloning.

#### Scenario: Roadmap groups items by time horizon

- **WHEN** a reader opens `docs/ROADMAP.md`
- **THEN** they see three sections minimum: near-term (next 1-2 releases), medium-term (quarter-ish), speculative (no timeline)
- **AND** each entry has a one-line description and, where relevant, a rough complexity tag (`S`/`M`/`L`)

#### Scenario: Roadmap covers known post-v0.1.0 candidates

- **WHEN** the reader scans the near-term and medium-term sections
- **THEN** they find at minimum: code signing + notarization for macOS, Linux port (gnome-keyring/secret-service), Homebrew tap, cargo-binstall support, more provider adapters with full usage parsing (OpenRouter, Tavily, CoinGecko), and token-budget policies
- **AND** a disclaimer at the top states "nothing here is committed"

#### Scenario: README points readers to the roadmap

- **WHEN** a reader scrolls `README.md`
- **THEN** a visible link near the bottom (or in an explicit "Roadmap" section) points to `docs/ROADMAP.md`
- **AND** the link text makes clear it is future-looking

### Requirement: v0.1.0 is cut and published

The first release SHALL be tagged as `v0.1.0` on `main` after the release-readiness checklist passes. The published GitHub Release MUST contain both macOS architecture tarballs and reference the CHANGELOG entry in its release notes.

#### Scenario: Pre-tag gate confirms shippable state

- **GIVEN** the maintainer has completed every item in `docs/RELEASE.md`'s checklist
- **WHEN** they are ready to push the tag
- **THEN** `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo fmt --all -- --check` all pass locally
- **AND** `CHANGELOG.md` contains a `## [0.1.0] - YYYY-MM-DD` section with accurate content
- **AND** the working tree is clean (`git status` reports no changes)

#### Scenario: Tag produces a GitHub Release with both architectures

- **WHEN** `git push origin v0.1.0` completes
- **THEN** the `.github/workflows/release.yml` run succeeds
- **AND** the resulting GitHub Release at `https://github.com/zengyuzhi/credential-broker/releases/tag/v0.1.0` lists exactly two tarball assets: `vault-aarch64-apple-darwin.tar.gz` and `vault-x86_64-apple-darwin.tar.gz`
- **AND** the release body contains both the auto-generated commit summary and a link or copy of the CHANGELOG `## [0.1.0]` section

#### Scenario: Install script resolves v0.1.0 end-to-end

- **GIVEN** the v0.1.0 GitHub Release is published
- **WHEN** a user on macOS runs `curl -fsSL https://raw.githubusercontent.com/zengyuzhi/credential-broker/main/install.sh | bash`
- **THEN** the script reports `Latest version: v0.1.0`, downloads the matching architecture tarball, installs `vault` to `~/.local/bin/`, and prints "Run 'vault --help' to get started."
- **AND** `vault --version` subsequently reports `vault-cli 0.1.0`

#### Scenario: Release failure aborts before user-visible damage

- **GIVEN** any checklist item fails (test red, clippy warning, dirty tree, CHANGELOG empty)
- **WHEN** the maintainer attempts to proceed
- **THEN** `docs/RELEASE.md` instructs them to stop, fix, and restart the checklist from the top
- **AND** no tag is pushed until every gate is green

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
