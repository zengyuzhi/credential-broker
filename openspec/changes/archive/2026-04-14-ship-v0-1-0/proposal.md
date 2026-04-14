## Why

Distribution infrastructure (binary rename, CI, release workflow, install script) shipped but no version has ever been tagged. "v0.1.0" sits in `Cargo.toml` and `vault --version` reports it, yet no GitHub release exists, no CHANGELOG tracks what's in it, and no written criteria says when it's ready to cut. This change sets the bar for the first release, actually cuts it, and writes down what comes after — so future releases have a template instead of a scramble.

Audience for v0.1.0 is the author only (personal use). Polish bar: shippable, not pristine. Rough edges allowed; they go in the CHANGELOG's "Known limitations" section rather than blocking the release.

## What Changes

- Add `CHANGELOG.md` at repo root following Keep-a-Changelog format; populate initial `[0.1.0]` entry enumerating everything shipped to date (credential CRUD, profile bindings, `vault run` injection, HTTP proxy, web dashboard, `vault serve`, `vault ui`, `vault stats`, CI, release workflow, install script).
- Add `docs/RELEASE.md` describing the evergreen release procedure: version bump location, tag format, what `git tag -a v* && git push --tags` triggers, how to verify the published release, how to test the install script against the live release.
- Add `docs/ROADMAP.md` listing post-v0.1.0 candidates (e.g., Linux/Windows ports, signing+notarization, Homebrew tap, more provider adapters, token-budget policies) grouped loosely by milestone. No commitments — reference only.
- Add a v0.1.0 release-readiness checklist (embedded in `docs/RELEASE.md`) and run it once: tests green, clippy clean, `vault --help` output reviewed, README install path works against a test tag, known limitations captured.
- Cut the actual tag: `v0.1.0` pushed to GitHub, release workflow runs, artifacts (`vault-aarch64-apple-darwin.tar.gz`, `vault-x86_64-apple-darwin.tar.gz`) appear on the Releases page, auto-generated notes supplemented with the CHANGELOG entry.

## Capabilities

### New Capabilities
- `release-process`: How credential-broker cuts and documents releases. Covers CHANGELOG conventions, the release-readiness checklist, the tag-and-push procedure, post-release verification steps, and the pointer to `docs/ROADMAP.md` for future work.

### Modified Capabilities
<!-- None — this adds a new capability; existing specs (binary-rename, github-release-ci, install-script) are not modified. -->

## Impact

- **New files**: `CHANGELOG.md`, `docs/RELEASE.md`, `docs/ROADMAP.md`
- **No code changes**: version stays at `0.1.0` (already correct in `crates/vault-cli/Cargo.toml`). No crate source files touched.
- **One external action**: push tag `v0.1.0` to GitHub; release workflow (already in place) produces the artifacts. This is the only step with visible-to-others blast radius — confirm before pushing.
- **Affected crates**: none (docs-only + one tag push)
- **Dependencies**: none added

## Security Implications

None. This change adds documentation and pushes a git tag. No secrets, auth, or credential-handling code is touched. The release workflow is already audited (archived in `add-distribution-packaging`) and builds with unprivileged GITHUB_TOKEN. The published binary contains no bundled secrets — all credentials remain in the user's Keychain.

## Out of Scope

- Linux / Windows support (roadmap only; v0.1.0 stays macOS-only)
- Code signing / notarization (tracked in roadmap; users on macOS 15+ will need `xattr -d com.apple.quarantine` until this lands)
- Homebrew tap / cargo-binstall manifest (roadmap)
- Automated CHANGELOG generation from conventional commits (manual for now)
- Marketing (announcement posts, landing page, screenshots) — personal release, no audience to announce to
- v0.2.0 scope commitments — `docs/ROADMAP.md` is a candidate list, not a plan of record
- Any behavioral change to the CLI, server, or dashboard
