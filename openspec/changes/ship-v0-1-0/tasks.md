## 1. CHANGELOG.md

- [x] 1.1 Create `CHANGELOG.md` at repo root with Keep-a-Changelog 1.1.0 header and `## [Unreleased]` section (empty subsections)
- [x] 1.2 Add `## [0.1.0] - YYYY-MM-DD` section (fill date on tag day); populate `### Added` with credential CRUD, profile bindings, `vault run` injection, HTTP proxy with lease auth, web dashboard (PIN auth, CSRF, SSE), `vault serve` (foreground/background/status/stop), `vault ui` auto-start, `vault stats` (`--json`, `--provider`), CI workflow, release workflow, `install.sh`
- [x] 1.3 Add "Known limitations" note to v0.1.0 entry: macOS-only, unsigned binary (quarantine attribute), personal-scale polish
- [x] 1.4 Add link references at the bottom of the file for each version tag pointing to the GitHub compare URLs (e.g. `[0.1.0]: https://github.com/zengyuzhi/credential-broker/releases/tag/v0.1.0`)

## 2. docs/RELEASE.md

- [x] 2.1 Create `docs/RELEASE.md` with overview paragraph (audience: maintainer; cadence: as-needed)
- [x] 2.2 Add "Versioning" section documenting single source of truth (`crates/vault-cli/Cargo.toml`), semver posture, and why other crates' versions aren't coordinated
- [x] 2.3 Add numbered release-readiness checklist: `cargo test --workspace` green; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo fmt --all -- --check` clean; CHANGELOG `[Unreleased]` moved to `[X.Y.Z] - <today>` with accurate bullets; `vault --help` output reviewed; version bumped in `crates/vault-cli/Cargo.toml`; clean `git status`; `main` branch up to date
- [x] 2.4 Add "Cut the release" section with copy-paste commands: commit docs, `git tag -a vX.Y.Z -m "Release X.Y.Z"`, `git push origin main`, `git push origin vX.Y.Z`
- [x] 2.5 Add "Verify" section: check Actions tab for release workflow success, confirm two tarball assets on the Releases page, run `install.sh` in a scratch shell and confirm `vault --version` matches
- [x] 2.6 Add "Rollback" section distinguishing pre-upload failure (safe to delete tag + retag same version) from post-upload failure (bump patch, retag new version); include `gh release delete`, `git push origin :refs/tags/vX.Y.Z`, `git tag -d vX.Y.Z` commands

## 3. docs/ROADMAP.md

- [x] 3.1 Create `docs/ROADMAP.md` with disclaimer at top: "Nothing here is a commitment. This is a candidate list, not a plan."
- [x] 3.2 Add `## Near-term` section with at minimum: code signing + notarization for macOS, `xattr` quarantine workaround in README, CHANGELOG enforcement in CI (fail if `[Unreleased]` empty at tag time)
- [x] 3.3 Add `## Medium-term` section with at minimum: Linux port (`secret-service` / `gnome-keyring`), Homebrew tap, cargo-binstall manifest, more provider adapters with full usage parsing (OpenRouter, Tavily, CoinGecko), token-budget policies
- [x] 3.4 Add `## Speculative` section with exploratory items (e.g., Windows port, multi-user mode, web dashboard with remote access, plugin architecture for adapters)
- [x] 3.5 Each bullet: one-line description plus rough `S`/`M`/`L` complexity tag where it adds signal

## 4. README.md

- [x] 4.1 Add visible "Roadmap" subsection (or a single line near the end of the "Development" section) linking to `docs/ROADMAP.md`
- [x] 4.2 Add CHANGELOG link in a sensible spot (likely near Installation or after Features) pointing to `CHANGELOG.md`
- [x] 4.3 Add "Release process" link or one-line mention pointing to `docs/RELEASE.md` in the Development section
- [x] 4.4 Add a note under Installation about Gatekeeper quarantine on modern macOS: `xattr -d com.apple.quarantine ~/.local/bin/vault` if the binary refuses to run

## 5. Pre-tag gate

- [x] 5.1 Run the freshly written checklist from `docs/RELEASE.md` end-to-end against the current repo state
- [x] 5.2 `cargo test --workspace` — 73 passed / 0 failed across vault-core (19), vault-providers (1), vault-secrets (9), vault-db (8+4), vault-telemetry (1), vault-policy (2), vault-cli (12+1), vaultd (16)
- [x] 5.3 `cargo clippy --workspace --all-targets -- -D warnings` — exit 0, zero warnings
- [x] 5.4 `cargo fmt --all -- --check` — clean after running `cargo fmt --all` to fix pre-existing unformatted code in 8 source files (vaultd/routes, vault-cli/commands, vault-db, etc.)
- [x] 5.5 `vault --help` reviewed — all 6 subcommands (credential, profile, run, stats, ui, serve) present and documented; no drift vs README Quick Start
- [x] 5.6 `crates/vault-cli/Cargo.toml` confirmed at `version = "0.1.0"`
- [x] 5.7 CHANGELOG v0.1.0 section dated `2026-04-14`
- [x] 5.8 Committed as 3 commits on `main`: `03b2267` (archive + spec sync), `d49c363` (style: cargo fmt), `a7d20ce` (docs: CHANGELOG/RELEASE/ROADMAP + README links + OpenSpec change). Fmt was split from docs commit since it touched source files unrelated to the release narrative.

## 6. Cut the tag

- [ ] 6.1 Confirm `git status` is clean and `git rev-parse --abbrev-ref HEAD` is `main`
- [ ] 6.2 `git push origin main`
- [ ] 6.3 `git tag -a v0.1.0 -m "Release 0.1.0 — initial personal release"`
- [ ] 6.4 `git push origin v0.1.0`

## 7. Post-tag verification

- [ ] 7.1 Watch the release workflow run in GitHub Actions; confirm all three jobs (test, build matrix, release) succeed
- [ ] 7.2 Open `https://github.com/zengyuzhi/credential-broker/releases/tag/v0.1.0` and confirm exactly two tarball assets: `vault-aarch64-apple-darwin.tar.gz` and `vault-x86_64-apple-darwin.tar.gz`
- [ ] 7.3 Edit the release body to prepend the v0.1.0 CHANGELOG entry above the auto-generated commit summary, and mention the quarantine workaround
- [ ] 7.4 In a scratch shell with a fresh `$PATH`, run `curl -fsSL https://raw.githubusercontent.com/zengyuzhi/credential-broker/main/install.sh | bash`; confirm output reports `Latest version: v0.1.0` and `vault --version` reports `vault-cli 0.1.0`
- [ ] 7.5 If anything fails at 7.1-7.4: follow the rollback section of `docs/RELEASE.md`

## 8. Close the loop

- [ ] 8.1 Archive this OpenSpec change via `/opsx:archive ship-v0-1-0` so `release-process` enters `openspec/specs/`
- [ ] 8.2 Append a retrospective note to `docs/RELEASE.md` (1-3 bullets) capturing anything surprising about the first cut — feeds the next release's checklist
