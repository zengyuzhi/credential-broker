## ADDED Requirements

### Requirement: Keychain read path silent-failure invariant SHALL be a documented known gap until SecItem-based replacement lands

`SecretStore::get` in `crates/vault-secrets/src/keychain.rs` SHALL read keychain items via the `security_framework::passwords::get_generic_password` API without wrapping the call in `SecKeychain::disable_user_interaction()`. On macOS 15.x with `security-framework` 3.7.x that guard returns `errSecAuthFailed` for items whose ACL permits silent access, breaking baseline `vault run` functionality; removing it trades the "unauthorized reads fail silently instead of prompting" invariant for correct behavior on permitted reads. The code site SHALL carry a comment pointing at the follow-up `keychain-acl-rewrite` change that will restore the invariant via `SecItemCopyMatching` with `kSecUseAuthenticationUINone`. The `audit-hardening` capability SHALL retain this requirement until that follow-up change explicitly REMOVES it.

#### Scenario: Keychain read succeeds against a permitted ACL

- **WHEN** `MacOsKeychainStore::get` is invoked for an item whose ACL
  lists the calling binary (the default case for credentials created via
  `put_with_access`) or is set to `-A` (allow-all)
- **THEN** the call returns `Ok(Zeroizing<String>)` containing the secret
- **AND** no GUI prompt is displayed

#### Scenario: Known gap — unauthorized read may prompt instead of failing

- **WHEN** a caller invokes `MacOsKeychainStore::get` against an item
  whose ACL neither lists the calling binary nor permits all apps
- **THEN** the operating system MAY display a keychain-access GUI prompt
  (rather than the call failing silently with `errSecAuthFailed`)
- **AND** this behavior is a documented known gap tracked in
  `docs/uat-runs/FINDINGS.md` under `UAT-FIND-002`-adjacent follow-up
- **AND** a future `keychain-acl-rewrite` change SHALL reinstate silent
  failure using `SecItemCopyMatching` with
  `kSecUseAuthenticationUINone`, and SHALL REMOVE this requirement as
  part of that rewrite

#### Scenario: Regression guard catches re-introduction of the offending guard

- **WHEN** a UAT run executes `UAT-SEC-004` (regression-guard entry
  added to `docs/UAT.md` as part of this change)
- **THEN** the entry performs a real `vault run --profile <name> --
  env | grep OPENAI_API_KEY` dispatch through the keychain get path
- **AND** the entry PASSes iff the env var is injected; FAILs at the
  `[AUTO:ANY]` tier if `SecKeychain::disable_user_interaction()` or an
  equivalent guard is re-introduced in `SecretStore::get`
- **AND** the failure blocks the release gate via the AUTO:ANY ≥95%
  threshold before any human walks the MANUAL golden-path entries
