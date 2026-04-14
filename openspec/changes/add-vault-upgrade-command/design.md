## Context

Post-v0.1.1, users have two ways to get a newer `vault` binary: re-run
`install.sh` (opaque, loses local state context) or build from source
(contributor-only). Neither is an honest upgrade path for a tool in
active release.

The v0.1.0 `--version` regression (UAT-FIND-007) is a pertinent
warning: a single release shipped with a broken surface, and the
only user-visible recovery was "re-download the installer". For a
tool that mediates OpenAI/Anthropic keys this is a credibility and
supply-chain-trust hole.

The canonical specs already anchor some of the pieces:

- `install-script/spec.md` — how a user gets a first binary.
- `release-process/spec.md` — what a release is.
- `github-release-ci/spec.md` — how tarballs are built per target.
- `audit-hardening/spec.md` — invariants the security surface must
  uphold.

This change threads `vault upgrade` through those existing
capabilities without disturbing them structurally.

## Goals / Non-Goals

**Goals:**
- Single-command upgrade: `vault upgrade` fetches the latest release,
  verifies checksum + minisign signature, and atomically replaces the
  currently running `vault` binary.
- Trust chain rooted in the minisign public key embedded at build time.
  Neither DNS hijack of `github.com`, nor a MITM, nor a release-asset
  tamper is enough to silently install a malicious binary.
- Refuse to run while `vault serve --background` is active so the
  running daemon never becomes a confused deputy mid-swap.
- Zero phone-home on non-`upgrade` invocations — explicit opt-in.

**Non-Goals:**
- Launch-time "upgrade available" notice — follow-up change
  `add-upgrade-launch-notice`.
- Auto stop/replace/restart of background daemons — policy (a) only.
- Cross-platform (Linux, Windows) — follows the rest of vault's macOS
  scope.
- Delta updates, incremental binary patching, channels other than
  "latest stable".
- Replacing `install.sh` — `install.sh` remains the canonical
  first-install path.

## Decisions

### Decision 1 — Minisign over cosign for day-one signing

**Choice:** use `minisign` (Ed25519, single public-key file) as the
release-signing primitive. Embed the public key in the `vault-cli`
binary via `include_bytes!`. Publish `SHA256SUMS.minisig` alongside
`SHA256SUMS` as a release asset.

**Why:**
- **Simple mental model.** One key, one signature file, one
  verification path. Fits on one line: "is this SHA file signed by
  our key?" For a credential broker that's exactly the question we
  want auditors to ask.
- **Zero third-party trust.** No Rekor, no OIDC, no Fulcio — the
  trust chain is (user downloads binary from trusted source once) →
  (that binary carries our pubkey forever) → (every future upgrade
  verifies against that pubkey). No cloud dependency, no offline
  failure modes.
- **Already proven in the wild.** minisign is what `homebrew/install`,
  `ocaml/opam` and others use. `rust-minisign` 0.7 is
  audit-readable (<1k LoC of Rust, no unsafe).
- **Key rotation is tractable.** The single-key model is simpler to
  rotate than a keyless sigstore chain — we just ship a new vault
  version via `install.sh` that embeds the new pubkey, and the next
  `vault upgrade` against the new pubkey succeeds.

**Signing ops — key MUST NOT land on CI.** The private key stays on
the maintainer's workstation (encrypted volume or yubikey). CI
produces an **unsigned draft release** with tarballs + `SHA256SUMS`.
The maintainer runs `scripts/sign-release.sh` locally — it downloads
`SHA256SUMS`, signs it with the offline key, uploads
`SHA256SUMS.minisig` back to the draft release, and promotes the
draft to public. A read-only CI post-publish verification step
refuses any published release lacking `SHA256SUMS.minisig`, using
only the checked-in public key (no key access). This means a CI
compromise alone cannot produce a signed malicious release; the
attacker would need to additionally compromise the maintainer's
workstation.

**Alternatives considered:**
- **Private key as GH Actions secret.** Rejected — a compromised CI
  workflow, runner, or release-job can sign a malicious tarball in
  the same environment the embedded public key trusts. Collapses
  the trust boundary the signing scheme is supposed to create.
- **cosign keyless (sigstore + OIDC).** Rejected for v0.1.x — the
  ambient trust on Rekor + Fulcio is more than we need and adds a
  network dep on sigstore infrastructure at verify time. Revisit if
  we ever ship a cross-org distribution.
- **GPG.** Rejected — too much ambient state (keyring), too many
  ways to misuse, `gpgv` is a wire-protocol museum piece for a 2025
  Rust binary.
- **No signing, checksum only.** Rejected — detects only transit
  corruption, not a compromised GH release. For a secrets tool we
  want cryptographic provenance on day one.

### Decision 2 — Atomic rename of `current_exe()` from a same-filesystem staging directory

**Choice:** on invocation, `vault upgrade` creates a per-process
staging directory as a sibling of `canonicalize(current_exe())`:
`<install-dir>/.vault-upgrade-<pid>/`. ALL intermediate artifacts —
`SHA256SUMS`, `SHA256SUMS.minisig`, the downloaded tarball, the
extracted binary — live inside this directory. On success, the
final step is `std::fs::rename(<staging>/vault.new, current_exe())`.
Same filesystem by construction → no `EXDEV`, no copy/delete
fallback, no partial-write window. A cleanup handler removes the
staging directory on every exit path.

**Why:**
- The running `vault upgrade` process holds the old inode; when
  `rename(2)` replaces it, the running process keeps executing from
  the old inode (now unlinked) until it exits. The next `vault …`
  invocation sees the new inode.
- No partial-write window where a user could see a half-written
  binary. Either the old inode or the new one is resolvable at any
  point.
- Using `$TMPDIR` (e.g., `/tmp`, which is commonly on a different
  APFS volume or even tmpfs) would risk cross-filesystem `rename(2)`
  failures and force a copy/delete fallback that violates the
  atomic-swap contract. Staging on the install filesystem is the
  only contract that's simultaneously verifiable, atomic, and
  implementable without a fallback path.

**Alternatives considered:**
- **Stage in `$TMPDIR`, copy to install dir on success.** Rejected —
  cross-filesystem `rename(2)` may fail with `EXDEV`, forcing
  copy-then-delete which reintroduces the partial-write window this
  change is trying to forbid.
- **Write to `<dir>/vault.new`, then `mv` via shell.** Rejected —
  adds a subprocess where a direct syscall works, and shell `mv` is
  the same `rename(2)` with argv parsing overhead.

### Decision 3 — Daemon-running refusal (policy a)

**Choice:** if `vault serve --background` is running (canonical PID file
at `state_dir().join("vault.pid")` resolves to a live process),
`vault upgrade` errors out with exit code 2 and prints:

```
error: vault daemon is running (pid 12345).
hint:  stop it first with `vault serve stop`, then retry `vault upgrade`.
       after upgrading, restart with `vault serve --background`.
```

**Why:**
- **Simplest and safest.** The daemon is stateful — in-flight lease
  tokens, open SQLite connections, an in-memory session map. An
  auto-restart would need to serialize that state across the swap,
  and any bug there directly costs users their in-flight traffic.
- **Explicit over clever.** The user is the authority on when to
  restart their daemon. `vault upgrade` is a surgical tool — it
  replaces a binary and gets out of the way.
- **Upgrade path to policy (b)** stays clean. When (b) lands in
  v0.2.x, this behavior becomes the `--allow-daemon-restart=false`
  default and nothing about the public surface breaks.

**Alternative considered:** policy (b), auto stop/replace/restart.
Rejected for this change — correctness bar too high for a
first-iteration.

### Decision 4 — Version comparison and downgrade guard

**Choice:** `vault upgrade` parses both the running version
(`env!("CARGO_PKG_VERSION")`) and the target version (GH release
`tag_name`, stripped of a leading `v`) as semver. Default behavior:
run only if `target > running`. `--force --to <ver>` bypasses the
comparison and lets you pin to an older or equal version.

**Why:**
- Default blocks downgrade attacks where an attacker redirects to
  an older, known-vulnerable release.
- `--force --to <ver>` keeps rollback possible during incident
  response without writing a separate "rollback" subcommand.
- Requires `--to` to be present when `--force` is used so the user
  cannot accidentally re-install the same version silently.

**Alternative considered:** `--force` alone allows any version.
Rejected — too easy to misuse; the `--to` requirement forces
intent.

### Decision 5 — Signature + checksum BOTH, in strict order

**Choice:** the verification pipeline is:

```
0. create staging dir <install-dir>/.vault-upgrade-<pid>/
   (register cleanup handler; same filesystem as current_exe() by construction)
1. fetch SHA256SUMS and SHA256SUMS.minisig into the staging dir
2. verify SHA256SUMS.minisig against embedded pubkey
   → on failure: abort with "signature verification failed"
3. fetch the target tarball (vault-<triple>.tar.gz) into the staging dir
4. compute SHA-256 of the tarball
5. compare to the corresponding line in SHA256SUMS
   → on failure: abort with "checksum mismatch"
6. extract to <staging>/extract/, verify the extracted binary is executable
7. move extracted binary to <staging>/vault.new (within-staging rename)
8. std::fs::rename(<staging>/vault.new, current_exe()) — same-fs atomic swap
```

Nothing is written to `current_exe()` before step 8. If any
verification step fails, the only filesystem artifact is the
per-process staging directory, which the cleanup handler removes on
every exit path. `current_exe()` itself is never touched on failure.

**Why:** strict order means the signature protects the entire
checksum file, and the checksum protects the specific tarball. A
compromised GH release that swaps the tarball will fail step 5; a
release that swaps the SHA file will fail step 2; a release that
swaps both will fail step 2 if the pubkey hasn't been compromised.

### Decision 7 — Unified absolute daemon state path (prerequisite migration)

**Choice:** introduce `support::config::state_dir() -> PathBuf`, an
absolute, cwd-independent resolver derived from the *resolved* DB URL
(`current_database_url()`), not from `std::env::current_dir()`.
Concretely: parse the filesystem path portion of the active SQLite URL,
take its parent directory, and use that as the state dir. Migrate
`serve`, `ui`, `stop`, and `upgrade` to consult
`state_dir().join("vault.pid")`. For pre-migration compatibility, add
only a **best-effort same-cwd fallback** for a legacy
`.local/vault.pid`; arbitrary historical cwd-relative PID files are not
globally discoverable and this change does not pretend otherwise.

**Why:** today `crates/vault-cli/src/commands/serve.rs::pid_file_path()`
returns `PathBuf::from(".local/vault.pid")` — a cwd-relative path.
A daemon started from `/tmp/foo` is invisible to a `vault serve
stop` or `vault upgrade` run from `$HOME/projects/bar`, because each
invocation resolves the PID file relative to its own current
working directory. For today's ergonomics this is a latent footgun
("`vault serve stop` from the wrong dir silently no-ops"); for this
change it is a security hole — `vault upgrade`'s daemon-running
refusal can be bypassed by running from a different cwd. Making
the path absolute closes the bypass for all *newly written* PID files
and fixes the latent bug in one go.

Because historical PID files were written relative to arbitrary working
directories, there is no single location we can reliably "migrate from".
The honest compatibility story is therefore best-effort only: if the
caller happens to run from the same directory that still contains a
legacy `.local/vault.pid`, we can inspect / clean it; otherwise the file
is simply not discoverable. The critical invariant for this change is
that every post-migration daemon writes only to `state_dir()/vault.pid`.

**Why this belongs in *this* change and not a separate one:**
the unified state path is a hard prerequisite for the daemon
refusal guard (Decision 3) to actually work. Splitting it into
`unify-daemon-state-path` would leave `add-vault-upgrade-command`
shipping a broken guard, and a staged rollout of
unify-then-upgrade doubles the overhead for a single-commit
amount of work.

**Alternatives considered:**
- **Use `$XDG_STATE_HOME` / `$HOME/.local/state/vault/`.** Rejected
  for v0.1.x because existing users have state at
  `<workspace-root>/.local/vault.db` (baked in at compile time via
  `CARGO_MANIFEST_DIR`). Changing both DB and PID paths at once is
  a data-migration question that deserves its own change.
- **Canonicalize the relative path at startup to absolute.**
  Rejected — "canonicalize a relative path" is `$cwd/.local/...`,
  which just freezes the bug at startup time. Doesn't fix cross-cwd
  invocations.
- **Lock the daemon via the DB file instead of a PID file.**
  Interesting, but SQLite's file lock doesn't survive process
  detach cleanly across platforms; PID-file + absolute path is the
  known-good pattern for long-lived macOS daemons.

### Decision 6 — No in-scope delta for `install-script`

**Choice:** `install.sh` is not modified by this change. A follow-up
task lists it as a future improvement (`install.sh` can fetch the
signature file and verify before extracting), but it is not a gate
for v0.2.0.

**Why:** `install.sh` is the bootstrap — it's the first time a user
ever sees our code, so the key they'd verify against has to come
from somewhere (out-of-band). That's a meaningful threat-model
conversation that is scoped better in a dedicated change.

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| Embedded pubkey is wrong / typo'd at build time | Build-time check: the crate's `build.rs` loads the pubkey file and parses it via `rust-minisign`; compile fails on invalid key. |
| minisign private key compromise | Key lives on the maintainer's workstation from day one (encrypted volume or yubikey) — never on CI, never as a GH Actions secret. Signing is a local `scripts/sign-release.sh` step between draft-release publish and public promotion. Key rotation is a `docs/RELEASE.md` runbook. |
| `rust-minisign` crate compromise | Pin exact version + `cargo vet` / `cargo audit` in CI. |
| GitHub outage blocks upgrades | `vault upgrade` fails gracefully with a stderr line; no retry loop, no fallback mirror (that would expand trust surface). User can retry later or reinstall via `install.sh`. |
| Users on macOS Ventura / Sonoma see different behavior from Sequoia | The upgrade path is pure filesystem ops + HTTP; no OS-specific APIs. Should behave identically across supported macOS versions. |
| `current_exe()` returns a symlink (e.g. `~/.local/bin/vault` symlinks into a `homebrew-cellar` path) | Canonicalize the path before choosing the staging-dir sibling; if the canonical path is under a system-managed prefix (brew, mac app bundle), refuse and recommend the package manager's update command. |
| Staging directory creation fails (EACCES on install dir, EROFS on a read-only mount) | Fail fast with a stderr message naming the install dir and the specific errno. Do not retry, do not fall back to `$TMPDIR` (would re-introduce `EXDEV`). User fixes the permission or reinstalls via `install.sh`. |
| Atomic rename succeeds but the new binary segfaults on next run | User re-runs `install.sh` to recover; documented in the error surface. A future `vault upgrade --to <previous>` is the in-band recovery path. |

## Migration Plan

1. Generate the minisign keypair on the maintainer's workstation,
   commit the public key to `crates/vault-cli/release-pubkey.minisign`,
   and update CI to publish unsigned draft releases with
   `SHA256SUMS` ready for the local signing handoff.
2. Implement `vault upgrade` with `--check`, `--to`, `--force`,
   `--dry-run` flags and the verification pipeline from Decision 5.
3. Write `cargo test` coverage for: version comparison, checksum
   mismatch, signature mismatch, daemon-running refusal, happy path
   (using a stub signer and a fake GH release URL).
4. Add a UAT entry (`UAT-UPG-001`, `[MANUAL:SHELL]`) that walks
   install v0.2.0 → tag v0.2.1 → `vault upgrade` → confirm
   `vault --version` reports v0.2.1.
5. First exercised on the v0.2.0 → v0.2.1 transition.

**Rollback:** `git revert` the change commit. The release pipeline
keeps publishing SHA + signature files unconditionally (harmless if
no `vault upgrade` ships). Embedded pubkey in older binaries stays
embedded but is unused until `vault upgrade` is re-introduced.

## Open Questions

- ~~**Where does the minisign private key live for v0.2.0?**~~ —
  **Resolved.** Key lives on the maintainer's workstation from day
  one (encrypted volume or yubikey). CI produces unsigned draft
  releases; signing is a local step. Rationale: putting the key on
  CI collapses the trust boundary — a CI compromise would let an
  attacker sign a malicious tarball against the embedded pubkey, so
  CI-stored keys are strictly worse than no signing. See Decision 1.
- ~~**Should `vault upgrade` record upgrade events in the usage_events
  DB?**~~ — **Resolved: no, not in this change.** The existing
  `UsageEvent` contract is provider- and credential-centric, and adding
  an upgrade-specific `metadata_json` payload would require a schema +
  model expansion outside this change's scope. This change keeps
  provenance user-visible (stderr key hash, release-body key hash) and
  leaves admin-event logging to a follow-up.
- **Should we log the verified pubkey ID at upgrade time?** Yes — print
  the key's short hash to stderr during `vault upgrade` so the user
  can compare it to the docs' canonical key ID out-of-band.
