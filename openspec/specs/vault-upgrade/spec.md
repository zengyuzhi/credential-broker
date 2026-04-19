## Purpose

Define how `vault upgrade` discovers, verifies, and installs newer direct-download `vault` binaries safely on macOS.

## Requirements

### Requirement: `vault upgrade` SHALL replace the installed binary only after checksum AND minisign signature both verify

`vault upgrade` SHALL fetch the latest release's `SHA256SUMS` and `SHA256SUMS.minisig` from the canonical GitHub release endpoint, verify the signature against the minisign public key embedded in the running binary, compute the SHA-256 of the downloaded tarball, and compare it to the corresponding line in the signed `SHA256SUMS` file. The installed binary MUST NOT be modified until both verifications succeed. If either check fails, `vault upgrade` MUST abort with exit code 3 and leave the installed binary untouched.

#### Scenario: Successful upgrade path

- **WHEN** `vault upgrade` is invoked with a newer release available on GitHub AND the release's `SHA256SUMS.minisig` verifies against the embedded public key AND the downloaded tarball's SHA-256 matches the line in `SHA256SUMS`
- **THEN** the extracted binary is renamed atomically over `std::env::current_exe()`
- **AND** stdout reports `upgraded <old-version> → <new-version>`
- **AND** the process exits 0

#### Scenario: Signature mismatch aborts before any filesystem change

- **WHEN** `vault upgrade` fetches a `SHA256SUMS.minisig` that does not verify against the embedded public key
- **THEN** the process exits with code 3 and stderr reads `signature verification failed: <minisign error>`
- **AND** `std::env::current_exe()` is byte-identical to its pre-invocation state
- **AND** the only filesystem artifact is the per-process staging directory next to `current_exe()`, which is removed before the process exits

#### Scenario: Checksum mismatch aborts before any filesystem change

- **WHEN** the `SHA256SUMS.minisig` verifies but the tarball's SHA-256 does not match the corresponding line in `SHA256SUMS`
- **THEN** the process exits with code 3 and stderr reads `checksum mismatch for <asset>: expected <sha>, got <sha>`
- **AND** `std::env::current_exe()` is byte-identical to its pre-invocation state

### Requirement: `vault upgrade` SHALL refuse to run while a `vault serve --background` daemon is active

`vault upgrade` MUST check the PID file at the absolute path `vault_cli::support::config::state_dir().join("vault.pid")` — the same path `vault serve --background`, `vault serve stop`, and `vault ui` consult, so the guard is effective regardless of the working directory of either invocation. If the file exists AND resolves to a running process whose executable path matches the current `vault` binary, `vault upgrade` MUST exit with code 2 and print a stop-hint to stderr before touching any release artifact.

#### Scenario: Daemon running — refuse with exact stop command

- **WHEN** a `vault serve --background` daemon is running and `vault upgrade` is invoked
- **THEN** the process exits with code 2
- **AND** stderr contains the pid of the running daemon
- **AND** stderr contains the literal command `vault serve stop`
- **AND** no HTTP request to the GitHub release endpoint is made

#### Scenario: Stale PID file — upgrade proceeds

- **WHEN** `state_dir().join("vault.pid")` exists but the referenced pid is not running
- **THEN** `vault upgrade` treats the daemon as not running and proceeds with the verification pipeline

#### Scenario: Daemon started from a different cwd is still detected

- **WHEN** a user ran `vault serve --background` from `/tmp/foo` AND later runs `vault upgrade` from `$HOME/projects/bar`
- **THEN** `vault upgrade` resolves the PID file via the absolute `state_dir()` path shared by `serve`
- **AND** the daemon-running refusal fires with exit code 2 (the guard is NOT bypassable by changing the working directory)

### Requirement: `vault upgrade` SHALL refuse to install a target version older than or equal to the running version without an explicit `--force --to <ver>`

The default path rejects downgrade and same-version reinstall. The `--force` flag MUST be accompanied by `--to <version>`; `--force` alone MUST NOT bypass the guard.

#### Scenario: Default downgrade attempt rejected

- **WHEN** the running binary is v0.2.5 and `vault upgrade` resolves the latest release as v0.2.4 (or a user runs `vault upgrade --to v0.2.4` without `--force`)
- **THEN** the process exits with code 4 and stderr reads `refusing to downgrade <running> → <target> without --force --to <target>`

#### Scenario: Explicit pinned downgrade succeeds

- **WHEN** the user runs `vault upgrade --force --to v0.2.4` against a v0.2.5 running binary
- **THEN** the verification pipeline runs for the pinned tarball
- **AND** on success the binary is replaced with v0.2.4

### Requirement: `vault upgrade --dry-run` and `--check` SHALL describe the action without mutating `current_exe()`

`vault upgrade --dry-run` MUST run the full verification pipeline and report what would be installed, then exit without the atomic rename. `vault upgrade --check` MUST query the release endpoint, compare versions, and report availability without downloading the tarball. Neither flag MUST produce any filesystem mutation at `std::env::current_exe()`. Both MUST remove any per-process staging directory before exit.

#### Scenario: `--dry-run` verifies and reports without installing

- **WHEN** `vault upgrade --dry-run` is invoked with a newer release available and verification succeeds
- **THEN** stdout reads `would upgrade <old> → <new> (checksum OK, signature OK by key <short-hash>)`
- **AND** `std::env::current_exe()` is byte-identical to its pre-invocation state

#### Scenario: `--check` reports availability without downloading tarball

- **WHEN** `vault upgrade --check` is invoked and a newer release exists
- **THEN** stdout reads `update available: <current> → <latest>`
- **AND** the process exits 0
- **AND** no tarball has been downloaded (only `releases/latest` JSON is fetched)
