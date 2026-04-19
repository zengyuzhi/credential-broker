## ADDED Requirements

### Requirement: `vault upgrade` SHALL root its trust chain in the minisign public key embedded in the running binary

The `vault upgrade` implementation in `crates/vault-cli/src/commands/upgrade.rs` MUST verify every candidate release against the minisign public key embedded at build time from `crates/vault-cli/release-pubkey.minisign`. The implementation MUST NOT accept a signing key sourced from any other location (no environment variable, no HTTP download, no filesystem path outside the binary). A release whose `SHA256SUMS.minisig` does not verify against the embedded key MUST be rejected before any tarball is downloaded or any filesystem write occurs at the install location.

#### Scenario: Embedded pubkey is the sole trust root

- **WHEN** `vault upgrade` starts verifying a release's signature
- **THEN** the verifier constructs its public key exclusively from the bytes returned by `include_bytes!("../release-pubkey.minisign")`
- **AND** no environment variable, command-line flag, or filesystem path is consulted to source the public key

#### Scenario: Build fails on malformed pubkey asset

- **WHEN** `crates/vault-cli/build.rs` parses `release-pubkey.minisign`
- **THEN** a parse failure or a file-absent condition causes the build to abort with a human-readable error
- **AND** no `vault-cli` binary is produced that lacks a verifiable embedded pubkey

### Requirement: `vault upgrade` SHALL NOT touch the installed binary on any verification failure

Neither a checksum mismatch, a signature mismatch, a network failure, nor a tarball-extraction error MAY cause any write or unlink to the path resolved by `std::env::current_exe()`. Intermediate artifacts (download buffers, `SHA256SUMS`, `SHA256SUMS.minisig`, the extracted binary) MUST live inside a per-process staging directory created as a sibling of `current_exe()` on the same filesystem (e.g., `<install-dir>/.vault-upgrade-<pid>/`), so that the terminal `rename(2)` into `current_exe()` is always a same-filesystem operation (no `EXDEV`, no copy/delete fallback). The only mutation of `current_exe()` allowed MUST be that terminal atomic rename after all checks have succeeded. The staging directory MUST be removed on every exit path (success, failure, panic).

#### Scenario: Any verification failure preserves the installed binary byte-for-byte

- **WHEN** `vault upgrade` aborts due to any verification failure (signature, checksum, network, extraction, or platform mismatch)
- **THEN** `std::env::current_exe()` is byte-for-byte identical to its pre-invocation state
- **AND** no file outside the per-process staging directory has been created or modified by `vault upgrade`
- **AND** the per-process staging directory is removed before the process exits

#### Scenario: Only the final rename mutates the install location

- **WHEN** `vault upgrade` reaches the end of the verification pipeline successfully
- **THEN** exactly one mutation of the install location occurs — `std::fs::rename(<staging-dir>/vault.new, current_exe())`
- **AND** the staging directory and `current_exe()` share the same filesystem (no `EXDEV`, no copy/delete fallback)
- **AND** no other write, truncate, copy-then-rename pattern, or shell subprocess is used at the install path
