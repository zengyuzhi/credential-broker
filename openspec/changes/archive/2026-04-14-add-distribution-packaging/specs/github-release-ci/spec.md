## ADDED Requirements

### Requirement: CI builds release binaries on version tag
A GitHub Actions workflow SHALL build release binaries for macOS (aarch64 and x86_64) when a tag matching `v*` is pushed. The binaries SHALL be uploaded as assets to a GitHub Release.

#### Scenario: Tag push triggers release
- **WHEN** a tag like `v0.1.0` is pushed to the repository
- **THEN** GitHub Actions builds `vault` binary for macOS aarch64 (Apple Silicon)
- **AND** builds `vault` binary for macOS x86_64 (Intel)
- **AND** creates a GitHub Release with both binaries as downloadable assets
- **AND** the release includes a changelog or tag message as the body

#### Scenario: Release asset naming
- **WHEN** binaries are uploaded to the release
- **THEN** they follow the pattern `vault-<target>.tar.gz` (e.g. `vault-aarch64-apple-darwin.tar.gz`)
- **AND** each tarball contains the `vault` binary

#### Scenario: CI runs tests before release
- **WHEN** the release workflow runs
- **THEN** `cargo test` and `cargo clippy` pass before building release binaries
- **AND** if tests fail, no release is created

### Requirement: CI on pull requests
A GitHub Actions workflow SHALL run `cargo test`, `cargo clippy`, and `cargo fmt --check` on every pull request and push to main.

#### Scenario: PR checks
- **WHEN** a pull request is opened or updated
- **THEN** tests, clippy, and format checks run automatically
- **AND** the PR shows pass/fail status
