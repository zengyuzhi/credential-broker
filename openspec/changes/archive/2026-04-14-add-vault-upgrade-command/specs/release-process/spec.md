## ADDED Requirements

### Requirement: Every tagged release SHALL publish a minisign-signed checksum manifest alongside the per-target tarballs

The release pipeline MUST publish two additional assets with every GitHub release tagged `v*`: a plain-text `SHA256SUMS` file listing one `<sha>  <asset>` line per per-target tarball (matching the `sha256sum` command's output format), and a `SHA256SUMS.minisig` file containing a detached minisign signature over `SHA256SUMS` produced with the offline release-signing key. Both assets MUST be part of the same release — the release MUST NOT be marked non-draft until both exist.

#### Scenario: Release assets include signed checksum manifest

- **WHEN** a release tagged `vX.Y.Z` is published via the release pipeline
- **THEN** `https://github.com/zengyuzhi/credential-broker/releases/download/vX.Y.Z/SHA256SUMS` returns a text file with one line per published tarball, each formatted as `<64-hex-sha>  vault-<target-triple>.tar.gz`
- **AND** `https://github.com/zengyuzhi/credential-broker/releases/download/vX.Y.Z/SHA256SUMS.minisig` returns a detached minisign signature that verifies against the public key distributed in `crates/vault-cli/release-pubkey.minisign`

#### Scenario: Signature is produced offline — never on CI

- **WHEN** a `vX.Y.Z` release is produced
- **THEN** the signing MUST be performed on the maintainer's workstation via `scripts/sign-release.sh` against the offline private key
- **AND** the private key MUST NOT be stored as a GitHub Actions secret, environment variable, or any other CI-accessible location
- **AND** the signature MUST be produced with the key whose public half is the one embedded in `vault-cli`

#### Scenario: CI publishes a draft release that a trusted signer promotes

- **WHEN** CI finishes building the per-target tarballs and computes `SHA256SUMS`
- **THEN** CI publishes these as assets of a **draft** release (not public)
- **AND** the release is NOT promoted to public until `SHA256SUMS.minisig` has been uploaded by the maintainer's local signing step
- **AND** a CI post-publish verification step (read-only; uses only the embedded public key) refuses any public release whose assets do not include a `SHA256SUMS.minisig` that verifies against `release-pubkey.minisign`

### Requirement: The release engineer SHALL record the public key's short hash in the release body

Every release description MUST include the minisign public key's short fingerprint (the `RWRxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx` prefix) next to the signing step note. This allows out-of-band key verification by users who want to cross-check the embedded key's identity.

#### Scenario: Release body cites signing key short hash

- **WHEN** a `vX.Y.Z` release is published
- **THEN** its release body (markdown) contains a line matching the pattern `signed by minisign key \`RW[A-Za-z0-9+/=]+\``

### Requirement: Key rotation SHALL be a documented runbook in `docs/RELEASE.md`

`docs/RELEASE.md` MUST contain a "Key rotation" section describing how to replace the minisign keypair. The runbook MUST cover: (1) generating the new keypair, (2) committing the new public key to `crates/vault-cli/release-pubkey.minisign`, (3) shipping a new vault release signed by the OLD key that embeds the NEW public key, (4) destroying the old private key after the first post-rotation release ships.

#### Scenario: `docs/RELEASE.md` has Key rotation section

- **WHEN** a reader opens `docs/RELEASE.md`
- **THEN** there is a second-level heading "Key rotation" whose body enumerates at least the four steps listed above in the stated order
