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
