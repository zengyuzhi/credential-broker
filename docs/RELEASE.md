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

4. **Security audit pass.**
   Re-run the Trail of Bits audit trio and compare against the previous baseline.
   Invoke each skill from a Claude Code session:

   - `zeroize-audit@trailofbits/skills`
   - `supply-chain-risk-auditor@trailofbits/skills`
   - `sharp-edges@trailofbits/skills`

   Create a new dated baseline directory following the `security-audit-baseline` convention:
   ```
   docs/audits/<YYYY-MM-DD>-<slug>/
   ```
   Populate with one per-skill file (provenance header + raw output) plus a `SUMMARY.md`
   that assigns severity (CRITICAL/HIGH/MEDIUM/LOW/INFO) and disposition
   (Fix now / Triage / Accept) per finding. Update `docs/audits/README.md` "Latest baseline"
   pointer in the same commit. Full rules live in
   [`../audits/README.md`](../audits/README.md) and the archived
   `openspec/specs/security-audit-baseline/spec.md`.

   **Gate decision (comparative, not absolute):**
   - If any CRITICAL or HIGH finding is **new** relative to the previous baseline → stop.
     Fix, Triage, or Accept (with rationale) the new finding before continuing.
   - If the new baseline is equal-or-lower risk versus prior (same findings or fewer, or
     same count at lower severities) → step passes.
   - A first-ever baseline has no prior to compare against; unresolved `Fix now`
     CRITICAL/HIGH items block archive per the capability spec.

   **Skip exception:** docs-only releases (no Rust source diff since the prior baseline)
   may skip re-running the trio. Record the skip in the Retrospective section below with
   the phrase "Security audit skipped: no code delta since `<prior-baseline-path>`".

   **Partial results:** if one skill errors/stalls, commit the partial output and flag the
   gap in `SUMMARY.md` under "Incomplete scans". Gate only uses findings that completed.
   If no skill completed, the step fails.

5. **CHANGELOG current.**
   - Open `CHANGELOG.md`.
   - Move everything under `## [Unreleased]` into a new section: `## [X.Y.Z] - YYYY-MM-DD` (today, UTC).
   - Re-create an empty `## [Unreleased]` block with empty `### Added / Changed / ... / Security` subsections at the top.
   - Add a "Known limitations" paragraph to the new version's entry if relevant.
   - Update the compare-URL footer: add a `[X.Y.Z]: .../releases/tag/vX.Y.Z` line and update `[Unreleased]: .../compare/vX.Y.Z...HEAD`.
   - If `## [Unreleased]` was empty before you started, stop and audit `git log` against the last tag — something got missed.

6. **`vault --help` sanity check.**
   ```bash
   cargo run -p vault-cli -- --help
   cargo run -p vault-cli -- run --help
   cargo run -p vault-cli -- serve --help
   ```
   Visually diff against README snippets. If drift exists, either update README or add a follow-up task. Drift alone does **not** block the release.

7. **Version bump.**
   - Edit `crates/vault-cli/Cargo.toml`: set `version = "X.Y.Z"`.
   - Run `cargo check -p vault-cli` to refresh `Cargo.lock`.
   - For the v0.1.0 initial release only: no bump needed (already at `0.1.0`).

8. **Working tree clean.**
   ```bash
   git status
   ```
   After staging the CHANGELOG + version bump commit, expect `nothing to commit, working tree clean` before tagging.

9. **On `main` and up to date.**
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

- **Pre-tag gate earned its keep on run #1.** `cargo fmt --check` caught 8 source files with formatting drift that none of our prior commits had normalized. CI would have caught it later; the checklist caught it before the tag went out. Keep the checklist.
- **Post-tag install-script smoke test surfaced a clap gap.** `vault --version` wasn't wired up (`#[command(version)]` missing), even though README / CHANGELOG / release body all advertise the flag. Binary itself works; only the documented version probe fails. Not a binary recall. Fix went into `[Unreleased] → Fixed` and will ship in v0.1.1. **Add `vault --version` to step 5 (help sanity check) for next release — not just as a README cross-check.**
- **GitHub Actions Node.js 20 deprecation warning.** `actions/checkout@v4` and `actions/upload-artifact@v4` run on Node 20, forced to Node 24 in June 2026. Non-blocking now; add "bump actions to v5 when available" to the roadmap's Near-term bucket.
- **Workflow total time: ~6 min end-to-end.** test (3m) → build matrix in parallel (2-3m each) → release (10s). Good budget; no need for build splits yet.

### Baseline audits

#### 2026-04-14 — Trail of Bits trio (first baseline)

- **26 findings across 3 skills.** `SUMMARY.md` at `docs/audits/2026-04-14-tob-baseline/SUMMARY.md`. Counts: 1 CRITICAL, 9 HIGH, 8 MEDIUM, 5 LOW, 3 INFO. Dispositions: 4 Fix-now (CRITICAL + 3 HIGH), 16 Triage, 6 Accept.
- **Value proven on the first pass.** The CRITICAL (non-CT PIN compare) and a HIGH (rate-limit keyed on a spoofable header) were real security-affecting defects in shipped v0.1.0 code. Without the trio they'd have sat in place until a user noticed or an external audit happened. One-shot, caught two.
- **Zeroize adoption is the biggest remaining work item.** Five HIGH findings (ZA-0001..0005) + SE-05 all point to the same root cause: no `zeroize` crate anywhere. Triaged as a standalone change (`add-zeroize-to-secret-paths`) rather than inlined — proper zeroize wiring touches every crate that handles secrets and deserves its own design doc.
- **Askama fork is archived.** Supply-chain scanner flagged `askama 0.12` / `askama_axum 0.4` upstream as archived. Migration is Medium-term; the current pin is there to bridge an axum 0.8 compat gap so replacing it is architecture, not renaming.
- **Scanner interactive-only.** None of the three skills run headlessly in CI today. The release-checklist step (4) is necessarily human-driven. A future change should investigate CLI-wrapper versions that'd fit in GitHub Actions.

---

Related: [CHANGELOG.md](../CHANGELOG.md), [ROADMAP.md](./ROADMAP.md), [.github/workflows/release.yml](../.github/workflows/release.yml).
