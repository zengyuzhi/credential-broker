## Context

credential-broker is a Rust workspace producing two binaries (`vault-cli`, `vaultd`). There's no CI, no release process, no install path beyond `cargo build`. The project is macOS-only due to Keychain integration.

## Goals / Non-Goals

**Goals:**
- Users install with one command (no Rust toolchain needed)
- Automated release process triggered by git tags
- CI catches regressions on every PR

**Non-Goals:**
- crates.io publishing (not yet — needs namespace consideration)
- Linux/Windows builds (macOS-only)
- Code signing / notarization (future)

## Decisions

### 1. Rename binary to `vault` via [[bin]] in Cargo.toml

Rationale: Users expect to type `vault`, not `vault-cli`. The `[[bin]]` section decouples the package name (used by Cargo for dependency resolution) from the output binary name. The package stays `vault-cli` to avoid namespace conflicts.

```toml
[[bin]]
name = "vault"
path = "src/main.rs"
```

### 2. Two GitHub Actions workflows: CI + Release

Rationale: Separate concerns. `ci.yml` runs on PRs and pushes to main (test + clippy + fmt). `release.yml` runs on `v*` tags (build + upload). Both use `macos-latest` runners since the codebase requires macOS.

CI workflow:
- Trigger: push to main, pull_request
- Steps: checkout, install Rust (stable), cargo fmt --check, cargo clippy, cargo test

Release workflow:
- Trigger: push tags `v*`
- Matrix: `[aarch64-apple-darwin, x86_64-apple-darwin]`
- Steps: checkout, install Rust + target, cargo build --release --target, tar + gzip, create GitHub Release, upload assets
- Uses `softprops/action-gh-release` for release creation

### 3. Cross-compilation via `cargo build --target`

Rationale: GitHub's `macos-latest` runners are Apple Silicon (M1+). Building for x86_64 requires `--target x86_64-apple-darwin` which cross-compiles via Xcode's toolchain. No need for separate runners.

### 4. Install script at `install.sh` in repo root

Rationale: Standard `curl | bash` pattern. The script:
1. Detects arch via `uname -m` (arm64 → aarch64, x86_64 → x86_64)
2. Fetches latest release tag from GitHub API
3. Downloads the matching tarball
4. Extracts to `~/.local/bin` (creates if needed)
5. Checks PATH and suggests export if needed

Using `~/.local/bin` (not `/usr/local/bin`) avoids requiring sudo.

### 5. Asset naming: `vault-{target}.tar.gz`

Rationale: Standard Rust release naming. Each tarball contains just the `vault` binary. Examples:
- `vault-aarch64-apple-darwin.tar.gz`
- `vault-x86_64-apple-darwin.tar.gz`

## Risks / Trade-offs

- [Risk] macOS runners are slower and more expensive on GitHub Actions → Only 2 targets, acceptable
- [Risk] Cross-compiled x86_64 binary may have subtle issues → Can test in CI via Rosetta
- [Risk] `vault` name conflicts with HashiCorp Vault → Our binary is local-only; users unlikely to have both. README should note the distinction.
