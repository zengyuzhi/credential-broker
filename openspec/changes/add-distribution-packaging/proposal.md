## Why

credential-broker currently requires `git clone` + Rust toolchain + `cargo build` to install. This is a developer-only workflow — normal users and teammates can't easily try it. We need a frictionless install path so people can go from zero to `vault credential add` in under a minute.

## What Changes

Three layers of distribution, each building on the previous:

1. **`cargo install` support** — rename binary to `vault`, add metadata to Cargo.toml
2. **GitHub Releases with prebuilt binaries** — CI workflow builds macOS aarch64/x86_64 on tag push, uploads to GitHub Release
3. **Install script** — `curl | bash` one-liner that detects platform, downloads the right binary, installs to `~/.local/bin`

## Out of Scope

- Homebrew tap (Level 4 — depends on GitHub Releases being stable first)
- Linux/Windows support (macOS-only due to Keychain dependency)
- Auto-update mechanism
- Signing/notarization (future enhancement for Gatekeeper)

## Capabilities

### New Capabilities
- `binary-rename`: CLI binary outputs as `vault` instead of `vault-cli`
- `github-release-ci`: Automated release workflow triggered by version tags
- `install-script`: Platform-detecting install script for one-command setup

### Modified Capabilities
- `cargo-install`: Proper metadata for `cargo install` from git/crates.io
