## Why

The v0.1.1-pre UAT walk (`docs/uat-runs/2026-04-14-v0.1.1-pre.md`) landed two
HIGH-severity regressions that block the `uat-release-gate` thresholds for any
v0.1.1 tag:

- **UAT-FIND-001 (HIGH):** `SecKeychain::disable_user_interaction()` in
  `crates/vault-secrets/src/keychain.rs` returns `errSecAuthFailed` on
  macOS 15 + `security-framework` 3.7, breaking `vault run` reads for every
  credential. Collapses the golden-path 4/4 requirement (UAT-CLI-002/003/004
  only pass on a modified working tree).
- **UAT-FIND-005 (HIGH):** The dashboard `/api/events` SSE stream does not
  push credential-state changes (e.g. `vault credential disable` from CLI).
  Rows stay `enabled` in the browser forever, violating the
  `dashboard-sse` "change surfaces within 4 s" contract. Pushes the
  MANUAL:USER UAT pass rate to 83% — one more regression and the gate
  flips to FAIL.

Without this change, v0.1.1 cannot pass step 5 of `docs/RELEASE.md`.
Deferring either fix to v0.1.2 leaves the first post-UAT release shipping
a broken golden path.

## What Changes

- Remove the `SecKeychain::disable_user_interaction()` guard from
  `SecretStore::get` (and the now-unused `SecKeychain` import) in
  `crates/vault-secrets/src/keychain.rs`. Document in a code comment and in
  the `audit-hardening` spec that a non-regressing replacement (via
  `SecItemCopyMatching` + `kSecUseAuthenticationUINone`) is deferred to a
  later release.
- Fix the dashboard SSE fan-out for credential-state transitions so
  `disable` / `enable` / `remove` via the CLI or the `/api/credentials/{id}/toggle`
  endpoint appears in any open dashboard tab within 4 seconds without a
  page refresh.
- Add a regression-guard UAT entry for the keychain read path
  (`UAT-SEC-004`, `[AUTO:ANY]`) so an in-process `vault run` smoke test
  catches any future return of the regression before tag push.

## Capabilities

### New Capabilities
<!-- none — this is a corrective change, not a feature addition -->

### Modified Capabilities

- `audit-hardening`: ADD one requirement that documents the known gap on
  the keychain read path (no `disable_user_interaction()` silent-fail
  invariant on macOS 15 + `security-framework` 3.7) and names the
  follow-up `keychain-acl-rewrite` change that will close it. Includes a
  regression-guard scenario tied to the new `UAT-SEC-004` entry. See
  `specs/audit-hardening/spec.md` in this change.

`dashboard-sse` is **not** modified: its canonical spec already contains
the scenario "CLI credential change appears in SSE within 4 seconds"
that UAT-DASH-003 exercises. This change brings the code into compliance
with that existing contract rather than altering the contract itself.

## Impact

- **Crates:** `vault-secrets` (one-line removal + import cleanup),
  `vaultd` (SSE handler + template row swap).
- **Tests:** `cargo test --workspace` must still report ≥ 73 passing
  (UAT-SEC-002 baseline); a new integration test SHOULD exercise the
  keychain get path against an `-A`-scoped item so the regression is
  caught at the crate level (not only at UAT).
- **Docs:** `docs/uat-runs/FINDINGS.md` UAT-FIND-001 and UAT-FIND-005
  transition to `Fixed` with this change's commit hash.
- **Release:** unblocks v0.1.1 tag. UAT-INSTALL-001 / UAT-FIND-007
  self-resolve once the tag is cut.

## Security Implications

- **Attack surface of the relaxed invariant:** removing
  `disable_user_interaction()` means that, in principle, a keychain item
  whose ACL does not permit silent access by the calling binary would
  trigger a GUI prompt instead of a silent failure. In the vault
  workload the calling binary is always on the ACL via
  `MacOsKeychainStore::put_with_access`, so no prompt is expected in
  practice. The proposal documents this as a known gap in
  `audit-hardening`'s canonical spec until the SecItem-based replacement
  lands.
- **No secrets are moved, logged, or re-keyed** by this change. The SSE
  fix does not change what data flows over the channel — it only
  ensures existing change events are actually fanned out to subscribers.
- **CSRF posture unchanged.** UAT-SEC-001 continues to protect all
  mutating endpoints; the SSE channel is read-only.

## Out of scope

- **UAT-FIND-002** (`security add-generic-password -w` stdin piping
  fallback) — needs an API rewrite (candidate: SecItem + SecAccess
  binding). Deferred to `keychain-acl-rewrite` change.
- **UAT-FIND-003** (`VAULT_LEASE_TOKEN` dual-UUID shape) — decision
  pending on "fix format" vs "document format"; deferred.
- **UAT-FIND-004** (Home-page sidebar collapses to a vertical strip) —
  cosmetic; does not fail any UAT entry. Deferred to `polish-ui`.
- **UAT-FIND-006** (stats rollup lumps `process_launch` under
  `provider=vault`) — cosmetic today; deferred to the same `polish-ui`
  change.
- **UAT-FIND-007** (v0.1.0 `--version` regression) — already fixed on
  `main` via commit `5990f12`; resolves the moment the v0.1.1 tag
  ships the existing binary.
- Batch 4 paid-provider UAT (`UAT-PROXY-OAI-001` / `ANT-001` /
  `TWAPI-001`) — run post-tag against the release binary; not gated on
  this change.
