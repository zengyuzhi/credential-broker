## 0. Prerequisite — unify daemon state path (absolute, cwd-independent)

- [x] 0.1 Add `pub fn state_dir() -> std::path::PathBuf` to `crates/vault-cli/src/support/config.rs`. It MUST return an absolute path whose resolution is cwd-independent: derive from the *resolved* database URL returned by `current_database_url()` (strip the `sqlite:` prefix, discard any `?query` suffix, take the parent directory). The helper MUST reuse the same DB-location semantics the CLI already uses, and MUST NOT read `std::env::current_dir()`.
- [x] 0.2 Create the state directory on first use with mode `0700`; on macOS this is `fs::create_dir_all` + `set_permissions`.
- [x] 0.3 Migrate `crates/vault-cli/src/commands/serve.rs::pid_file_path()` to return `config::state_dir().join("vault.pid")` instead of `PathBuf::from(".local/vault.pid")`. Write PID files at mode `0600`.
- [x] 0.4 Audit all other callers of `.local/vault.pid` (grep for the literal string) and migrate each to `state_dir().join("vault.pid")`. Expect hits in `vault ui` auto-start path and any test harness.
- [x] 0.5 Implement **best-effort** legacy-path handling, not a fake global migration: if `.local/vault.pid` exists relative to the caller's current working directory, `serve status` / `serve stop` / `upgrade` MAY inspect it as a legacy fallback when the canonical `state_dir()/vault.pid` is absent. If the legacy file is stale, remove it; if it points at a live daemon and the caller is in that same directory, honor it for that invocation. Do NOT claim that arbitrary historical cwd-relative PID files are discoverable from every directory.
- [x] 0.6 Add an integration test that starts a daemon from `cwd=/tmp/some-dir-1`, runs `vault serve status` from `cwd=/tmp/some-dir-2`, and asserts the status reports the live daemon (today this fails because status reads `./local/vault.pid` relative to cwd — this test is the regression guard).
- [x] 0.7 Add an integration test that invokes `vault upgrade --check` from a different `cwd` than the one the daemon was started from, asserts exit code 2 and the daemon-running refusal message (regression guard for Codex finding #3).
- [x] 0.8 Update `docs/UAT.md`'s `UAT-SERVE-001` entry (or add `UAT-SERVE-004`) to exercise the cross-cwd lifecycle start → stop path.

## 1. Release-signing infrastructure (offline key never lands on CI)

- [x] 1.1 Generate a minisign keypair on the maintainer's local workstation, never on CI (`minisign -G -p release-pubkey.minisign -s <kept-offline>`); record the public key's short hash for `docs/RELEASE.md` and the release-body signing note. The private key file (`~/.minisign/vault.key` or equivalent) stays on the maintainer's machine (ideally on an encrypted volume or a yubikey); it MUST NOT be committed and MUST NOT be uploaded as a GH Actions secret.
- [x] 1.2 Commit `crates/vault-cli/release-pubkey.minisign` (public half only). The file MUST be checked in verbatim so every commit of `vault-cli` embeds the same key.
- [x] 1.3 Add `crates/vault-cli/build.rs` that parses the pubkey at compile time via `rust-minisign`'s `PublicKey::from_base64`; fail the build on missing-file or parse-error.
- [x] 1.4 Configure the GH Actions `release` workflow to produce **unsigned** artifacts (tarballs + `SHA256SUMS`) into a **draft** release. CI MUST NOT have access to the private signing key and MUST NOT attempt to sign; the draft release is the handoff point from CI to the trusted signer.
- [x] 1.5 Add a local `scripts/sign-release.sh` (maintainer-run, documented in `docs/RELEASE.md`) that: (a) downloads `SHA256SUMS` from the draft release, (b) runs `minisign -Sm SHA256SUMS -s ~/.minisign/vault.key` locally, (c) uploads `SHA256SUMS.minisig` to the draft release via `gh release upload`, (d) promotes the draft to public only after the signature asset is present and re-verifies locally with `minisign -Vm SHA256SUMS -p crates/vault-cli/release-pubkey.minisign`.
- [x] 1.6 Add a CI **post-publish verification** step (pull request + post-release) that refuses a release lacking `SHA256SUMS.minisig`, using only the checked-in public key: pure read-only guard that detects "published without signing" regressions; does NOT require key access.
- [x] 1.7 Add a "Key rotation" second-level heading to `docs/RELEASE.md` enumerating the four-step runbook from the `release-process` spec.
- [x] 1.8 Add a "Signing flow" second-level heading to `docs/RELEASE.md` describing the draft-release handoff: CI produces draft → maintainer signs locally via `scripts/sign-release.sh` → maintainer promotes draft to public.

## 2. `vault upgrade` command scaffold

- [x] 2.1 Add `commands::upgrade` module in `crates/vault-cli/src/commands/upgrade.rs`; register `Upgrade` variant on the top-level `Commands` enum in `main.rs` with `--check`, `--to <version>`, `--force`, `--dry-run` flags.
- [x] 2.2 Add `rust-minisign` dep to `crates/vault-cli/Cargo.toml` pinned to an exact version; re-run `cargo vet` / `cargo audit` to re-baseline.
- [x] 2.3 Wire the embedded pubkey via `include_bytes!("../release-pubkey.minisign")`; parse once per invocation via a `OnceLock` guard.

## 3. Verification pipeline

- [x] 3.1 Implement GitHub-API client for `GET /repos/zengyuzhi/credential-broker/releases/latest` (and `GET /releases/tags/v<ver>` for `--to`); hardcode the repo slug — no env-var override.
- [x] 3.2 Resolve the staging directory as a sibling of `canonicalize(current_exe())`: `<install-dir>/.vault-upgrade-<pid>/`. Create it with `0700` permissions. MUST be on the same filesystem as `current_exe()` (precondition — the install dir IS that filesystem); fail fast with a clear error if creation is rejected (EACCES, EROFS).
- [x] 3.3 Register a staging-directory cleanup handler that fires on every exit path: success, early abort (signature / checksum / daemon guard / downgrade guard / platform guard), and panic. Implementation candidate: `scopeguard::defer!` + a `Drop` wrapper on the staging handle.
- [x] 3.4 Download `SHA256SUMS` and `SHA256SUMS.minisig` into the staging directory (no `$TMPDIR` fallback — all artifacts live next to `current_exe()`).
- [x] 3.5 Implement minisign verification against the embedded pubkey; on failure exit code 3 with the exact stderr line from the `vault-upgrade` spec.
- [x] 3.6 Implement tarball download into the staging directory + SHA-256 computation; on mismatch exit code 3 with the exact stderr line from the spec.
- [x] 3.7 Implement tarball extraction into `<staging>/extract/`; verify the extracted binary has the exec bit; refuse otherwise.
- [x] 3.8 Move the verified binary to `<staging>/vault.new` (rename within the staging dir), then `std::fs::rename(<staging>/vault.new, current_exe())`. Same-filesystem precondition is guaranteed by the sibling-dir layout, so `EXDEV` cannot occur. Only run this terminal rename after steps 3.5, 3.6, 3.7 all succeed.
- [x] 3.9 Implement canonical-path guard: if `canonicalize(current_exe())` is under a known package-manager prefix (brew, `/Library/Frameworks`, MacOS app bundle), refuse with a redirect to that package manager's update command.

## 4. Guards

- [x] 4.1 Implement daemon-running check via `state_dir().join("vault.pid")`; on active daemon, exit code 2 with the `vault serve stop` hint (verbatim as in the `vault-upgrade` spec). As a compatibility fallback only, if the canonical path is absent and a legacy `.local/vault.pid` exists in the caller's current working directory, inspect that file too.
- [x] 4.2 Implement downgrade guard: default path rejects `target <= running`; `--force` is only honored in conjunction with `--to <version>`; exit code 4 on guard trip.
- [x] 4.3 Implement platform guard: refuse on non-Darwin; exit code 5 with a message pointing at the GitHub README.

## 5. Dry-run + check

- [x] 5.1 `vault upgrade --check`: fetch `releases/latest` JSON only; print `update available: <current> → <latest>` (or `already on latest version: <current>`); no tarball download.
- [x] 5.2 `vault upgrade --dry-run`: run the full verification pipeline and pre-install guards (release lookup, staging dir, signature verification, checksum verification, extraction, package-manager/path guard), but skip the terminal `std::fs::rename(<staging>/vault.new, current_exe())`; print `would upgrade <old> → <new> (checksum OK, signature OK by key <short-hash>)`.

## 6. Operator-visible provenance

- [x] 6.1 At the top of every `vault upgrade` run, print `signing key: <short-hash>` to stderr so the user can cross-check against the docs' canonical key ID.
- [x] 6.2 Update the release-body handoff in `docs/RELEASE.md` so the maintainer records `signed by minisign key <short-hash>` when promoting the draft release to public.

## 7. Tests

- [x] 7.1 Unit: version-comparison helper rejects downgrades, accepts upgrades, rejects equal version, accepts equal version only with `--force --to <ver>`.
- [x] 7.2 Integration: a fixture GH-release JSON, a fixture `SHA256SUMS`, a fixture `SHA256SUMS.minisig` produced by a test-only key pair — verify happy path, signature-mismatch path, checksum-mismatch path against the verification pipeline without any real network call.
- [x] 7.3 Integration: `vault upgrade` refuses when a fake PID file points at the test's own PID (daemon-running refusal); stale PID file is treated as absent.
- [x] 7.4 Integration: `--dry-run` leaves a scratch binary byte-identical before and after the full verification pipeline.
- [x] 7.5 `cargo test --workspace --quiet` aggregate count stays ≥ 80 (current UAT-SEC-002 baseline 73 + 7 new tests from 7.1–7.4).

## 8. UAT coverage

- [x] 8.1 Add `UAT-UPG-001 — vault upgrade happy path`, type `[MANUAL:SHELL]`, cap `vault-upgrade`. The Cmd walks a two-release dance: `install.sh` v0.2.0 → tag v0.2.1 → `vault upgrade` → `vault --version` reports `vault 0.2.1`.
- [x] 8.2 Add `UAT-UPG-002 — vault upgrade signature-mismatch refusal`, type `[AUTO:ANY]`, cap `vault-upgrade`. The Cmd points `vault upgrade` at a local fixture server that serves a tampered `SHA256SUMS.minisig`; exit code MUST be 3 and the installed binary MUST be byte-identical before and after.
- [x] 8.3 Verify `grep -c '^#### UAT-' docs/UAT.md` stays within the 20–35 window defined by the `uat-release-gate` spec.

## 9. Docs

- [x] 9.1 Add a "Upgrading" section to `README.md` describing `vault upgrade`, `--check`, `--dry-run`, the daemon-running hint, and the downgrade-attack guard.
- [x] 9.2 Add an entry to `CHANGELOG.md` under the next-release `Added` section.
- [x] 9.3 Update `docs/RELEASE.md` to include the signing step after the tag-triggered draft release exists and before the maintainer promotes that release to public.

## 10. Release dry-run

- [ ] 10.1 Cut a v0.2.0-rc.1 tag in a test repo or via a draft release to exercise the full signing pipeline end-to-end without publishing; verify `SHA256SUMS` and `SHA256SUMS.minisig` both appear in the release assets and the signature verifies locally via `minisign -Vm SHA256SUMS -p release-pubkey.minisign`.
- [ ] 10.2 From a clean machine, `install.sh` v0.2.0-rc.1, then `vault upgrade --check` against a pretend v0.2.0 tag, then `vault upgrade` — confirm end-to-end.

## 11. Commit + ship

- [x] 11.1 Single atomic commit: `feat(vault-cli): add vault upgrade subcommand with minisign-verified self-update`.
- [x] 11.2 `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` — all green locally before push.
- [ ] 11.3 `git push origin main`; confirm CI green on the commit.
- [ ] 11.4 `/opsx:archive add-vault-upgrade-command`; verify the canonical `vault-upgrade` spec lands in `openspec/specs/vault-upgrade/spec.md` and that `release-process` + `audit-hardening` each carry the new requirements.
