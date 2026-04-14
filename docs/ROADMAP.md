# Roadmap

> **Nothing here is a commitment.** This is a candidate list, not a plan. Items can be promoted, demoted, deleted, or ignored without notice. When an item lands in a release, it moves into [CHANGELOG.md](../CHANGELOG.md) and out of this file in the same PR.

Ordering within a bucket is insertion order (newest at top). Complexity tags are rough — `S` = an afternoon, `M` = a weekend, `L` = a week-plus.

## Near-term

Things plausibly happening in the next one or two releases.

- **Code signing + notarization for macOS** `L` — sign the release binary with a Developer ID certificate and run it through Apple's notary service so Gatekeeper stops quarantining downloads. Requires an enrolled Apple Developer account and integration with the release workflow.
- **CHANGELOG enforcement in CI** `S` — fail the `check` job on `main` if `## [Unreleased]` would be empty at tag time, or add a lint that refuses release commits unless the version being released has a populated entry.
- **`vault --help` parity check** `S` — scripted diff between `vault --help` output and the Quick Start block in README, surfaced as a CI warning rather than an error.
- **`xattr` quarantine hint in README install section** `S` — document the workaround inline rather than only in release notes so first-time users find it.
- **Bump GitHub Actions to v5 when available** `S` — `actions/checkout@v4` and `actions/upload-artifact@v4` run on Node 20, which is forced to Node 24 in June 2026 and removed September 2026. Surfaced by v0.1.0 release workflow annotation.
- **Add `cargo-audit` to CI** `S` — RustSec advisory DB scan on `Cargo.lock`; catches public CVEs in the dep tree. Trail-of-Bits flavored; reviewed during v0.1.0 shipping.
- **Adopt `zeroize` across secret paths** `L` — API keys live in `String` / `Vec<u8>` and drop without wiping. Wrap `SecretStore::get` return in `Zeroizing<String>`; derive `ZeroizeOnDrop` on `ResolvedCredential` + lease raw token + dashboard session token. Covers ZA-0001..0005, ZA-0006, ZA-0007, SE-05. (audit: zeroize-audit + sharp-edges 2026-04-14)
- **Validate `ttl_minutes` in `issue_lease`** `S` — switch to `NonZeroU32` or return `Result` on zero/negative. (audit: sharp-edges 2026-04-14 SE-07)
- **Return error on timestamp overflow** `S` — stop silently substituting `now` in `checked_add_signed` fallbacks in `auth.rs` and `ui_sessions.rs`; callers should handle the failure. (audit: sharp-edges 2026-04-14 SE-06)
- **Sanitize `VAULT_TRUSTED_APP_PATHS` entries** `S` — require absolute path + file-existence check before passing to `/usr/bin/security`. (audit: sharp-edges 2026-04-14 SE-11)
- **Disambiguate missing-vs-empty header extraction** `S` — current code `unwrap_or("")` on Origin and auth header values, conflating absent with empty. Return 401 on absent, distinct from mismatch. (audit: sharp-edges 2026-04-14 SE-12)
- **Saturate `u128 as i64` latency cast** `S` — use `try_from` with `.unwrap_or(i64::MAX)` in proxy telemetry. (audit: sharp-edges 2026-04-14 SE-08)

## Medium-term

Things that would take a clear chunk of work and might or might not happen this quarter.

- **Linux port** `L` — back `SecretStore` with `secret-service` (GNOME Keyring / KWallet). Largely cfg-gating existing code and porting the trusted-app ACL ceremony. Keychain-specific code in `vault-secrets` would need an abstraction seam.
- **Homebrew tap** `M` — publish a formula at `zengyuzhi/homebrew-vault` pointing at release tarballs so `brew install zengyuzhi/vault/vault` works. Complements the curl-pipe installer without replacing it.
- **cargo-binstall manifest** `S` — add `[package.metadata.binstall]` so `cargo binstall vault-cli` pulls the prebuilt binary from GitHub Releases instead of compiling from source.
- **Full proxy adapters for OpenRouter, Tavily, CoinGecko** `M` — current code supports inject + basic forwarding; promote these to full adapters with response parsing for usage tracking.
- **Token-budget policies** `M` — per-profile monthly spend ceiling; `vault run` refuses to launch if the profile is over budget. Data already in `vault-telemetry`.
- **Dashboard search / filtering across all pages** `S` — text filter on Credentials, Profiles, Sessions; extend the Stats provider filter pattern.
- **Migrate off archived `askama` fork** `L` — current pins `askama = "0.12"` + `askama_axum = "0.4"` target `djc/askama` which is archived upstream. Move to `askama-rs/askama` or `rinja`. Non-trivial because the specific `askama_axum 0.4` pin is there to bridge the axum 0.8 incompat (see `CLAUDE.md` gotcha). (audit: supply-chain-risk-auditor 2026-04-14 SC-01)
- **Switch `estimated_cost_usd` from f64 to integer microdollars** `M` — floating-point accumulates drift across thousands of rollup rows; use `i64` microdollars and document the unit. (audit: sharp-edges 2026-04-14 SE-09)
- **Evaluate `keyring` as `security-framework` alternative** `M` — cross-platform abstraction that'd help the Linux port while shrinking the solo-maintained-FFI surface. (audit: supply-chain-risk-auditor 2026-04-14 SC-03)

## Speculative

Longer-shot ideas. Listed so they stop reappearing in conversation. Promotion to near-term means someone actually wants to do the work.

- **Windows port** `L` — back `SecretStore` with Windows Credential Manager. Lower priority than Linux given the coding-agent audience.
- **Multi-user mode** `L` — today the database and Keychain are per-user; a shared-broker deployment (one host serving a small team) would need auth beyond PIN, per-user lease scoping, and an audit log.
- **Remote access to dashboard** `L` — today CORS and bind-address are locked to `127.0.0.1`. A tunneled access mode (e.g., via `tailscale serve` or `cloudflared`) with stronger auth would let the dashboard follow you between machines.
- **Plugin architecture for provider adapters** `L` — move `ProviderAdapter` implementations out-of-tree so new providers don't require a fork. Likely a dynamic-library or scripting (Lua, Rhai) seam.
- **Automatic release drafting from conventional commits** `M` — once commit discipline is established (it isn't), switch CHANGELOG generation to `git-cliff` or `release-plz`.
- **Encrypted audit log export** `M` — dump `usage_events` for a date range as an age-encrypted bundle for cold storage.
- **Provider cost prediction** `M` — pre-flight estimate of a request's cost shown in the dashboard before the agent sends it.
- **Monitor solo-maintained crate risk** `S` (recurring) — `reqwest`, `rpassword` each have one primary maintainer. Re-evaluate replacement if maintenance cadence degrades. Pair with supply-chain re-audit each release. (audit: supply-chain-risk-auditor 2026-04-14 SC-02, SC-04)
- **Validate `secret_ref` service prefix** `S` — `parse` currently discards the service half of `"service:account"`, so a malformed ref with the wrong service is treated as valid. Either round-trip-check or drop the prefix. (audit: sharp-edges 2026-04-14 SE-10)

---

Related: [CHANGELOG.md](../CHANGELOG.md), [RELEASE.md](./RELEASE.md).
