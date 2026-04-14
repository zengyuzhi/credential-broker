# Release Procedure

Maintainer-facing guide for cutting a credential-broker release. Audience: the author. Cadence: as-needed.

The goal is a boring release: run the checklist top-to-bottom, push a tag, verify the artifacts, move on. If any step is surprising, update this document afterwards.

---

## Versioning

Single source of truth: `crates/vault-cli/Cargo.toml` (`version = "X.Y.Z"`). That's the version `vault --version` reports and the version the GitHub release carries.

Other workspace crates (`vault-core`, `vault-db`, etc.) are **not** coordinated. They stay at `0.1.0` indefinitely — they are not published to crates.io, and bumping them would imply an API contract we don't maintain.

Semver posture: while on `0.y.z`, minor bumps (`0.1.z → 0.2.0`) are allowed to break CLI flags, env var names, or dashboard URLs. Patch bumps (`0.1.0 → 0.1.1`) are bugfix-only.

## Release-readiness checklist

Run every item. If any fails, stop, fix, and restart from the top.

1. **Tests green.**
   ```bash
   cargo test --workspace
   ```
   Expect the final line to report `test result: ok`.

2. **Clippy clean.**
   ```bash
   cargo clippy --workspace --all-targets -- -D warnings
   ```
   Expect zero output.

3. **Format clean.**
   ```bash
   cargo fmt --all -- --check
   ```
   Expect zero output.

4. **CHANGELOG current.**
   - Open `CHANGELOG.md`.
   - Move everything under `## [Unreleased]` into a new section: `## [X.Y.Z] - YYYY-MM-DD` (today, UTC).
   - Re-create an empty `## [Unreleased]` block with empty `### Added / Changed / ... / Security` subsections at the top.
   - Add a "Known limitations" paragraph to the new version's entry if relevant.
   - Update the compare-URL footer: add a `[X.Y.Z]: .../releases/tag/vX.Y.Z` line and update `[Unreleased]: .../compare/vX.Y.Z...HEAD`.
   - If `## [Unreleased]` was empty before you started, stop and audit `git log` against the last tag — something got missed.

5. **`vault --help` sanity check.**
   ```bash
   cargo run -p vault-cli -- --help
   cargo run -p vault-cli -- run --help
   cargo run -p vault-cli -- serve --help
   ```
   Visually diff against README snippets. If drift exists, either update README or add a follow-up task. Drift alone does **not** block the release.

6. **Version bump.**
   - Edit `crates/vault-cli/Cargo.toml`: set `version = "X.Y.Z"`.
   - Run `cargo check -p vault-cli` to refresh `Cargo.lock`.
   - For the v0.1.0 initial release only: no bump needed (already at `0.1.0`).

7. **Working tree clean.**
   ```bash
   git status
   ```
   After staging the CHANGELOG + version bump commit, expect `nothing to commit, working tree clean` before tagging.

8. **On `main` and up to date.**
   ```bash
   git rev-parse --abbrev-ref HEAD    # must print "main"
   git pull --ff-only origin main
   ```

## Cut the release

```bash
git add CHANGELOG.md crates/vault-cli/Cargo.toml Cargo.lock
git commit -m "chore: release vX.Y.Z"
git push origin main

git tag -a vX.Y.Z -m "Release X.Y.Z"
git push origin vX.Y.Z
```

The tag push is the point of no return for that version number. See [Rollback](#rollback) if something goes wrong after this.

## Verify

1. **Actions tab.** Go to `https://github.com/zengyuzhi/credential-broker/actions` and watch the `Release` workflow triggered by the tag. All three jobs (`test`, `build` matrix × 2, `release`) must complete green.

2. **Release assets.** Open `https://github.com/zengyuzhi/credential-broker/releases/tag/vX.Y.Z` and confirm exactly two tarballs are attached:
   - `vault-aarch64-apple-darwin.tar.gz`
   - `vault-x86_64-apple-darwin.tar.gz`

3. **Release notes.** The workflow auto-generates commit-level notes. Edit the release and prepend the `## [X.Y.Z]` section from `CHANGELOG.md` above the auto-generated block. For pre-1.0 releases, also paste the macOS quarantine workaround:
   > If macOS Gatekeeper refuses to run the binary: `xattr -d com.apple.quarantine ~/.local/bin/vault`

4. **Install script end-to-end.** In a scratch shell with a clean `$PATH`:
   ```bash
   PATH="/usr/bin:/bin" bash -c 'curl -fsSL https://raw.githubusercontent.com/zengyuzhi/credential-broker/main/install.sh | bash'
   ~/.local/bin/vault --version
   ```
   Expect the script to report `Latest version: vX.Y.Z` and `vault --version` to match.

## Rollback

Two cases — pick the right one:

### Case A: Failure before any artifact was uploaded

The release workflow failed during the `test` or `build` jobs, so no tarball exists at any URL. It is safe to delete the tag and retry with the same version number.

```bash
gh release delete vX.Y.Z --yes 2>/dev/null || true   # in case a draft exists
git push origin :refs/tags/vX.Y.Z
git tag -d vX.Y.Z
```

Fix the root cause, commit, and restart the "Cut the release" section.

### Case B: Failure after an artifact was uploaded (or might have been downloaded)

Once a tarball has been publicly reachable, caches (user machines, CI, CDNs) may have it. Reusing the tag creates two artifacts with the same version — a debugging landmine.

Bump the patch version and re-release instead:

```bash
gh release delete vX.Y.Z --yes           # remove the broken release from GitHub UI
git push origin :refs/tags/vX.Y.Z        # remove the remote tag
git tag -d vX.Y.Z                        # remove the local tag
# edit crates/vault-cli/Cargo.toml to the NEXT patch (X.Y.Z+1)
# add a short CHANGELOG entry explaining the skip
# re-run the full checklist, then cut vX.Y.Z+1
```

Add a one-line note to `CHANGELOG.md` under the new version: "Re-release of X.Y.Z; previous tarball withdrawn due to <reason>."

## Retrospective

After each release, append 1-3 bullets here noting anything surprising. Next release's checklist improvements come from this section.

### v0.1.0 (2026-04-14)
<!-- fill in after cutting -->

---

Related: [CHANGELOG.md](../CHANGELOG.md), [ROADMAP.md](./ROADMAP.md), [.github/workflows/release.yml](../.github/workflows/release.yml).
