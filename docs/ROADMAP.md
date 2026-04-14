# Roadmap

> **Nothing here is a commitment.** This is a candidate list, not a plan. Items can be promoted, demoted, deleted, or ignored without notice. When an item lands in a release, it moves into [CHANGELOG.md](../CHANGELOG.md) and out of this file in the same PR.

Ordering within a bucket is insertion order (newest at top). Complexity tags are rough — `S` = an afternoon, `M` = a weekend, `L` = a week-plus.

## Near-term

Things plausibly happening in the next one or two releases.

- **Code signing + notarization for macOS** `L` — sign the release binary with a Developer ID certificate and run it through Apple's notary service so Gatekeeper stops quarantining downloads. Requires an enrolled Apple Developer account and integration with the release workflow.
- **CHANGELOG enforcement in CI** `S` — fail the `check` job on `main` if `## [Unreleased]` would be empty at tag time, or add a lint that refuses release commits unless the version being released has a populated entry.
- **`vault --help` parity check** `S` — scripted diff between `vault --help` output and the Quick Start block in README, surfaced as a CI warning rather than an error.
- **`xattr` quarantine hint in README install section** `S` — document the workaround inline rather than only in release notes so first-time users find it.

## Medium-term

Things that would take a clear chunk of work and might or might not happen this quarter.

- **Linux port** `L` — back `SecretStore` with `secret-service` (GNOME Keyring / KWallet). Largely cfg-gating existing code and porting the trusted-app ACL ceremony. Keychain-specific code in `vault-secrets` would need an abstraction seam.
- **Homebrew tap** `M` — publish a formula at `zengyuzhi/homebrew-vault` pointing at release tarballs so `brew install zengyuzhi/vault/vault` works. Complements the curl-pipe installer without replacing it.
- **cargo-binstall manifest** `S` — add `[package.metadata.binstall]` so `cargo binstall vault-cli` pulls the prebuilt binary from GitHub Releases instead of compiling from source.
- **Full proxy adapters for OpenRouter, Tavily, CoinGecko** `M` — current code supports inject + basic forwarding; promote these to full adapters with response parsing for usage tracking.
- **Token-budget policies** `M` — per-profile monthly spend ceiling; `vault run` refuses to launch if the profile is over budget. Data already in `vault-telemetry`.
- **Dashboard search / filtering across all pages** `S` — text filter on Credentials, Profiles, Sessions; extend the Stats provider filter pattern.

## Speculative

Longer-shot ideas. Listed so they stop reappearing in conversation. Promotion to near-term means someone actually wants to do the work.

- **Windows port** `L` — back `SecretStore` with Windows Credential Manager. Lower priority than Linux given the coding-agent audience.
- **Multi-user mode** `L` — today the database and Keychain are per-user; a shared-broker deployment (one host serving a small team) would need auth beyond PIN, per-user lease scoping, and an audit log.
- **Remote access to dashboard** `L` — today CORS and bind-address are locked to `127.0.0.1`. A tunneled access mode (e.g., via `tailscale serve` or `cloudflared`) with stronger auth would let the dashboard follow you between machines.
- **Plugin architecture for provider adapters** `L` — move `ProviderAdapter` implementations out-of-tree so new providers don't require a fork. Likely a dynamic-library or scripting (Lua, Rhai) seam.
- **Automatic release drafting from conventional commits** `M` — once commit discipline is established (it isn't), switch CHANGELOG generation to `git-cliff` or `release-plz`.
- **Encrypted audit log export** `M` — dump `usage_events` for a date range as an age-encrypted bundle for cold storage.
- **Provider cost prediction** `M` — pre-flight estimate of a request's cost shown in the dashboard before the agent sends it.

---

Related: [CHANGELOG.md](../CHANGELOG.md), [RELEASE.md](./RELEASE.md).
