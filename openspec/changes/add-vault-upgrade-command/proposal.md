## Why

Today there is no in-band way to upgrade an installed `vault` binary.
Users either re-run `curl … | bash` against `install.sh` (opaque,
requires remembering the URL) or live with whatever version they
first installed. UAT-FIND-007 (v0.1.0 `--version` regression) is the
concrete proof-point: every user on v0.1.0 had a broken `vault
--version` and no observable path to v0.1.1 short of re-running the
installer.

This is untenable for a tool that holds API keys. An explicit
`vault upgrade` subcommand:
- gives users a one-command upgrade path,
- cryptographically binds each upgrade to our release signing key
  (minisign, day-one), and
- keeps all trust decisions explicit (refuses to run against a
  background daemon, refuses on checksum/signature mismatch, rejects
  unsupported platforms and architectures).

The follow-up `add-upgrade-launch-notice` change will add an
opt-out launch-time notice that nudges users to run `vault upgrade`
when a new release is available — but that change depends on this
one landing first.

## What Changes

- Add `vault upgrade` subcommand to `vault-cli` with flags:
  `--check` (detect without installing), `--to <version>` (pin),
  `--force` (allow same-or-older), `--dry-run`.
- Add release-pipeline steps to publish `SHA256SUMS` and
  `SHA256SUMS.minisig` as release assets alongside the tarballs.
- Embed the minisign **public** key in the binary (`vault-cli` build
  pulls it in via `include_bytes!`). The matching private key is held
  offline by the release engineer and **never lands on CI** — CI
  produces unsigned draft releases; signing happens on the
  maintainer's workstation via a local `scripts/sign-release.sh`; the
  draft is then promoted to public only after the signature asset
  lands. A read-only CI post-publish guard refuses any published
  release that lacks `SHA256SUMS.minisig`.
- Refuse to run `vault upgrade` when a `vault serve --background`
  daemon is running; print the exact stop command.
- Migrate the daemon PID file from the cwd-relative
  `.local/vault.pid` to an absolute, cwd-independent path returned
  by a new `support::config::state_dir()` helper. Required
  prerequisite — without it, the daemon-running refusal can be
  bypassed by running `vault upgrade` from a different working
  directory than the daemon was started in (Codex finding). Legacy
  pre-migration `.local/vault.pid` files are only discoverable
  best-effort from the caller's current working directory; this
  change does not claim arbitrary old cwd-relative PID files can be
  found globally.
- Document the supply-chain trust invariant in `audit-hardening`
  (every upgrade SHALL be checksum- AND signature-verified against
  the embedded pubkey before the binary swap).
- Extend `release-process` canonical spec to require the signing
  handoff after tag push: CI creates a draft release, the maintainer
  signs `SHA256SUMS` locally, and only then is the release promoted
  to public.

## Capabilities

### New Capabilities

- `vault-upgrade`: the `vault upgrade` subcommand, its flags, its
  error surface (daemon-running, checksum mismatch, signature
  mismatch, same-or-older version, network failure, unsupported
  platform), and its file-system contract (atomic rename of
  `current_exe()`).

### Modified Capabilities

- `release-process`: ADD a requirement that every tagged release
  SHALL publish `SHA256SUMS` and `SHA256SUMS.minisig` alongside the
  per-target tarballs, signed by the offline minisign key whose
  public half is embedded in the binary.
- `audit-hardening`: ADD a requirement documenting the `vault
  upgrade` trust invariant (checksum + signature verification must
  succeed before the atomic rename; a mismatch MUST abort without
  touching the installed binary).
- `vault-serve-lifecycle`: MODIFY the "Background mode with PID
  file" requirement — PID file path becomes absolute and
  cwd-independent via the new `state_dir()` helper, shared by
  `serve`, `ui`, `stop`, and `upgrade`. Includes a best-effort legacy
  compatibility path for same-cwd `.local/vault.pid` cleanup;
  arbitrary historical cwd-relative PID files are not globally
  discoverable.

## Impact

- **Crates:** `vault-cli` gains a new `commands/upgrade.rs` module,
  a minisign-verification dependency (candidate:
  `rust-minisign` 0.7.x), an `include_bytes!` for the embedded
  public key, a new `support::config::state_dir()` helper, and
  migrations of `commands/serve.rs` + any `ui.rs` auto-start path
  + stop command to consult that helper instead of a cwd-relative
  PID path.
- **CI / release pipeline:** `.github/workflows/release.yml` changes
  from "tag push => public release with tarballs" to "tag push =>
  unsigned draft release with tarballs + `SHA256SUMS`". The trusted
  maintainer signs `SHA256SUMS` locally, uploads
  `SHA256SUMS.minisig`, and then promotes the draft to public. CI
  remains keyless.
- **Docs:** `docs/RELEASE.md` gains a signing step after the
  tag-triggered draft release is created and before the maintainer
  promotes that release to public; `install.sh` optionally verifies
  the signature post-download (follow-up task, tracked on this
  change but not blocking).
- **Release channels:** v0.2.0 is the earliest release that will
  ship with `vault upgrade`. v0.1.x remains installable only via
  `install.sh`.

## Security Implications

- **New ambient surface:** one signed public endpoint
  (`api.github.com/repos/.../releases/latest`) and one signature
  verification per upgrade. `vault upgrade` itself is explicit,
  user-invoked, and logged to stderr — no ambient phone-home.
- **Key compromise blast radius:** if the minisign private key is
  stolen, every `vault upgrade` user is one release away from
  arbitrary code execution. Mitigations: key stored offline on the
  maintainer's workstation (never on CI, never as a GH secret), key
  rotation documented in `docs/RELEASE.md`, embedded key is
  revokable by shipping a new version via `install.sh`. A compromise
  of CI alone — without access to the maintainer's workstation —
  cannot produce a signed malicious release; the worst it can do is
  publish an unsigned draft, which the read-only post-publish guard
  refuses.
- **Downgrade-attack guard:** `vault upgrade` rejects a target
  version older than or equal to the running binary unless `--force`
  is supplied and `--to <version>` is given explicitly.
- **Daemon-running refusal:** protects the running process against
  binary/inode mismatch and against a confused-deputy scenario where
  the same `vault serve` process might be handling in-flight lease
  tokens during the swap.

## Out of scope

- **Launch-time "upgrade available" notice** — tracked as the next
  change (`add-upgrade-launch-notice`).
- **Daemon auto-stop/restart around upgrade** — policy (a) only for
  this change; smoother auto-restart is a v0.2.x follow-up.
- **Package-manager distribution** (Homebrew tap, cargo-binstall) —
  separate `add-homebrew-tap` change, orthogonal to this one.
- **Linux / Windows support** — `vault upgrade` binds to macOS
  (Darwin + aarch64/x86_64) the same way the rest of the tool does.
- **`install.sh` signature verification** — SHOULD follow this
  change, but is not a gate on it; `vault upgrade` is the primary
  signed-update path.
