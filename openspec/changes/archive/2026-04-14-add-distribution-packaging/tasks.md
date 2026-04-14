## Tasks

### Task 1: Rename binary to `vault` and add Cargo metadata

- [x] Add `[[bin]] name = "vault"` section with `path = "src/main.rs"`
- [x] Add `description`, `license = "MIT"`, `repository`, `keywords` fields
- [x] Verify `cargo build -p vault-cli` produces `target/debug/vault`
- [x] Update README Quick Start
- [x] Commit: `chore: rename CLI binary to vault and add Cargo metadata`

---

### Task 2: Add CI workflow for PRs and main

- [x] Create `.github/workflows/ci.yml`
- [x] Trigger on push to main and pull_request
- [x] Install Rust stable via `dtolnay/rust-toolchain@stable`
- [x] Run fmt check, clippy, test
- [x] Commit: `ci: add GitHub Actions CI workflow`

---

### Task 3: Add release workflow for version tags

- [x] Create `.github/workflows/release.yml`
- [x] Trigger on push tags `v*`
- [x] Matrix: `[aarch64-apple-darwin, x86_64-apple-darwin]`
- [x] Test before build, build release, package tarball
- [x] Upload via `softprops/action-gh-release@v2`
- [x] Commit: `ci: add GitHub Actions release workflow`

---

### Task 4: Create install script

- [x] macOS-only gate, arch detection, latest release fetch
- [x] Download + extract to `~/.local/bin/`
- [x] PATH check with guidance
- [x] Upgrade support (overwrites existing)
- [x] Commit: `feat: add one-command install script`

---

### Task 5: Update README with install instructions

- [x] Add Installation section with 3 options (curl, cargo install, manual)
- [x] Add HashiCorp Vault name distinction note
- [x] Commit: `docs: add installation instructions to README`

---

### Task 6: Verify

- [x] `cargo build --workspace` — clean
- [x] `cargo clippy --workspace --all-targets -- -D warnings` — clean
- [x] `cargo test --workspace` — 73 tests pass
- [x] `target/debug/vault --help` works
- [x] Reviewed CI and release workflows
- [x] Reviewed install.sh
