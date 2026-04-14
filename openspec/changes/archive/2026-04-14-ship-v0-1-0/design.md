## Context

The `add-distribution-packaging` change (archived 2026-04-14) built the rails: the binary is named `vault`, CI runs on every PR, a release workflow triggers on `v*` tags and uploads per-architecture macOS tarballs, and `install.sh` downloads the latest release into `~/.local/bin/`. None of those rails have carried a train yet. `crates/vault-cli/Cargo.toml` claims `version = "0.1.0"`, `vault --version` reports it, but `https://github.com/zengyuzhi/credential-broker/releases` is empty.

Audience for v0.1.0 is the author alone. This means: no announcement pressure, no SLA, no backwards-compat obligations once v0.2.0 ships. Polish bar is "works for me on my laptop"; known limitations go into the CHANGELOG rather than blocking the tag.

The change is fundamentally documentation + one external action (pushing a git tag). There is no Rust code to modify. The interesting design questions are all about conventions that must live past this release.

## Goals / Non-Goals

**Goals:**
- Make future releases boring: a checklist exists, procedure is documented, CHANGELOG format is decided, rollback steps are written down.
- Actually ship v0.1.0 — tag, artifacts, visible on the Releases page.
- Capture the post-v0.1.0 candidate list somewhere durable so it stops living in conversation context.
- Zero code changes. Docs + tag only.

**Non-Goals:**
- Automating the release. A human runs the checklist and pushes the tag. `cargo-release`, `release-plz`, `git-cliff`, and conventional-commits tooling are all deferred (roadmap).
- Code signing, notarization, or Apple Developer ID integration — users will strip the quarantine attribute manually for v0.1.0; proper signing is roadmap work.
- Linux/Windows — roadmap; v0.1.0 remains macOS-only.
- Release branches. Tags are cut directly from `main`; no `release/*` branches.
- Marketing / announcement. Personal release.
- Changing the existing CI or release workflows. They work; don't touch them.

## Decisions

### Decision 1: CHANGELOG format — Keep-a-Changelog 1.1.0

Use the Keep-a-Changelog 1.1.0 structure: `## [Unreleased]` at the top, then `## [X.Y.Z] - YYYY-MM-DD` sections descending, each grouping bullets under `### Added / Changed / Deprecated / Removed / Fixed / Security`.

**Why:** It's the de-facto standard for human-maintained changelogs, every contributor recognizes the shape, and it maps cleanly onto GitHub release notes when copy-pasted. The `Unreleased` convention gives a clear landing spot for in-flight changes so the next tag cut is mostly clerical.

**Alternatives considered:**
- *Auto-generated from conventional commits* (git-cliff / release-plz) — rejected for v0.1.0: too much tooling investment for a personal release, and our commit history predates any convention. Revisit in roadmap once commit discipline is established.
- *Free-form release notes in GitHub only* — rejected: notes then live only on GitHub, can't be grep'd from a clone, and the format drifts every release.
- *GitHub auto-generated release notes alone* — rejected as the sole source: useful as a supplement but they're just commit subjects, which tells users nothing about user-visible impact.

The release workflow already has `generate_release_notes: true`, so we get commit summaries for free. The CHANGELOG entry is copied into the release body on top of that.

### Decision 2: Release procedure lives in `docs/RELEASE.md`, not CONTRIBUTING.md or a wiki

Single dedicated file, checked into the repo, referenced from `README.md`'s Development section.

**Why:** The release procedure is read exactly when cutting a release — a `docs/` path beats a wiki (which can't be cloned offline and has a separate edit history) and beats stuffing it into `CONTRIBUTING.md` (which doesn't exist yet and covers a different concern — how to contribute vs. how to ship).

**Alternatives considered:**
- *GitHub wiki* — rejected: out-of-repo, version history diverges from code.
- *Inline in `README.md`* — rejected: bloats the README, release procedure is maintainer-facing, README is user-facing.
- *`CONTRIBUTING.md`* — rejected: wrong audience; contributing is for PR-writers, releasing is for the maintainer.

### Decision 3: Roadmap is a candidate list, not a plan

`docs/ROADMAP.md` groups ideas into Near-term / Medium-term / Speculative buckets with an explicit disclaimer at the top: "nothing here is a commitment." Each bullet gets a one-line description and optionally a rough S/M/L complexity tag.

**Why:** A public roadmap with dates is a promise. We don't want that liability for a personal project; we *do* want the idea-parking-lot so the "what's next?" discussion stops happening in conversation. Buckets + disclaimer let us list Linux port, signing, Homebrew, more adapters, etc., without anyone expecting a shipping quarter.

**Alternatives considered:**
- *GitHub Issues labelled `roadmap`* — deferred to roadmap itself: useful later when there are collaborators, overkill now.
- *GitHub Projects board* — rejected: too heavy for a personal backlog; harder to diff and review than a markdown file.
- *No roadmap file* — rejected: ideas keep resurfacing in conversation; writing them down is cheap.

### Decision 4: Version bumps are manual

Bump `version = "0.1.0"` in `crates/vault-cli/Cargo.toml` by hand, then tag. No `cargo-release`, no `cargo set-version`, no pre-commit hook.

**Why:** One workspace member has a version we care about (the binary); no dependents consume our crates. Automation's value kicks in when you have workspace-wide coordination. At one version field, `sed` is overkill.

**Alternatives considered:**
- *`cargo-release`* — rejected for v0.1.0, noted in roadmap.
- *`release-plz`* — rejected for same reason.
- *Version field in every crate* — rejected: we already have divergent per-crate versions (most are `0.1.0` but `vault-cli` is the user-visible one). Keeping them loose avoids the false impression that publishing `vault-db 0.2.0` to crates.io means anything.

### Decision 5: Tag directly from `main`, no release branch

Procedure: check out `main`, pull, run the checklist, commit any doc/version bumps, push to `main`, `git tag -a v0.1.0 -m "Release 0.1.0"`, `git push origin v0.1.0`.

**Why:** Release branches (`release/0.1`, `release/0.2`) exist to support parallel dev of N+1 while stabilizing N. We have one developer and no stabilization window. Branches would be ceremony without benefit.

**Alternatives considered:**
- *`release/*` branch per minor* — rejected: no parallel development happens.
- *Tag on a release candidate commit separate from `main`* — rejected: makes `git log main` disagree with release history.

### Decision 6: Bad-tag rollback prefers a new patch over reusing the tag

If a release workflow fails partway, or an artifact is discovered broken *after* anyone could have downloaded it: delete the GH Release and tag, bump patch (e.g., `0.1.0 → 0.1.1`), re-tag. Do not force-push or re-tag the same version.

**Why:** Once a binary tarball exists at a URL, some users' caches (install scripts, CI caches, personal `~/.local/bin/`) may have it. Reusing the tag creates two artifacts with the same version — a debugging disaster. Cheap patch bump > correctness hole.

Exception: if the failure happened *before* any artifact was uploaded (e.g., workflow failed at `cargo test`), the tag has no binary associated with it, so delete+retag is safe. `docs/RELEASE.md` calls this distinction out explicitly.

### Decision 7: CHANGELOG entry content — user-facing, not implementation

Bullets describe what changed from a user's perspective ("`vault ui` now auto-starts the server") not what changed in code ("refactored `spawn_background_server` into `serve.rs`"). Commit history is for the latter.

**Why:** Two audiences, two artifacts. The CHANGELOG answers "should I upgrade?"; `git log` answers "why did this change?". Conflating them makes both worse.

### Decision 8: v0.1.0 release notes will note macOS-only + quarantine caveat

The release body explicitly tells first-time downloaders: `xattr -d com.apple.quarantine ~/.local/bin/vault` if the binary refuses to run. No surprises, no support tickets.

## Risks / Trade-offs

- **[Pushing the tag is one-way]** → Mitigation: the checklist exists to prevent cutting with a dirty tree; the rollback section covers the "I pushed a bad tag" case with an explicit new-patch-bump path.
- **[Personal-scale polish bar leaks into v0.2.0]** → Mitigation: the "personal use only" posture is documented in the CHANGELOG's Known limitations section, so the next release can explicitly decide whether to raise the bar or stay personal.
- **[Manual CHANGELOG discipline rots]** → Mitigation: the release checklist in `docs/RELEASE.md` fails loudly if `## [Unreleased]` is empty or missing at release time, forcing a manual audit against `git log`.
- **[Roadmap becomes a wishlist that nobody prunes]** → Mitigation: keep it small. Pruning is cheaper than re-evaluating; when an item lands in a release, it moves to the CHANGELOG and out of the roadmap in the same PR.
- **[Install script references `main` branch]** → already mitigated: `install.sh` on `main` fetches the `latest` release via GitHub API, so as long as v0.1.0 is published, `main`'s script resolves it. No script change needed per release.
- **[Unsigned binary triggers Gatekeeper on modern macOS]** → Mitigation: documented in release notes; tracked in roadmap for a future signed release.

## Migration Plan

**Pre-tag (all local, reversible):**
1. Create `CHANGELOG.md`, `docs/RELEASE.md`, `docs/ROADMAP.md`.
2. Update `README.md` to link the roadmap.
3. Run the newly written checklist against the repo.
4. Commit docs in one commit on `main`.

**Tag push (external effect):**
5. `git tag -a v0.1.0 -m "Release 0.1.0"`
6. `git push origin v0.1.0` — this is the point of no return for this version number.
7. Watch the GitHub Actions release run. If it fails *before* uploading assets, delete the tag (`git push origin :refs/tags/v0.1.0` + `git tag -d v0.1.0`), fix, retag same version.

**Post-tag verification:**
8. Confirm two tarball assets on the Releases page.
9. Fresh-shell test: `curl … install.sh | bash` against a scratch `PATH`, confirm `vault --version` reports `0.1.0`.

**Rollback (if a bad artifact escapes):**
- Delete the GH Release + tag, bump to `0.1.1`, re-tag. Do not reuse `v0.1.0`.

## Open Questions

- **Should `CHANGELOG.md` cover pre-v0.1.0 history?** Leaning yes for the v0.1.0 section itself (everything shipped *is* v0.1.0), but no retroactive `[0.0.x]` entries. Final call at implementation time.
- **Does the v0.1.0 release body embed the full CHANGELOG entry or link to it?** Embed — users reading the Releases page shouldn't need a second click. Auto-generated commit summary appears below.
- **Roadmap sort order within a bucket: priority, alphabetical, or insertion?** Insertion (most recent idea at top). Re-sorting by priority is a trap; we'll promote items by moving them to the bucket above.
