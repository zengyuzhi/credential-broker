# Supply Chain Risk Audit — credential-broker

**Skill:** supply-chain-risk-auditor@trailofbits/skills  
**Invocation date:** 2026-04-14  
**Scope:** All `Cargo.toml` files in workspace (`Cargo.toml`, `crates/*/Cargo.toml`)  
**Tool version:** 1.0.0  
**Method:** GitHub API (`gh`) queries per dependency — star counts, open issues, contributor lists, SECURITY.md presence, RustSec advisory DB (rustsec/advisory-db)

---

## High-Risk Dependencies

| Dependency | Pinned Version | GitHub Repo | Stars | Risk Factors | Suggested Alternative |
|---|---|---|---|---|---|
| `askama` / `askama_axum` | 0.12.1 / 0.4.0 | djc/askama (**archived** → askama-rs/askama) | ~3,500 / ~1,000 | Upstream archived. `djc/askama` archived Mar 2025; successor at `askama-rs/askama` (~1,000 stars, active as of Apr 2026). Pinned versions come from dead release line. No SECURITY.md on successor. `askama_axum` 0.4 is intentionally pinned for axum 0.8 compat but that line receives no security patches. | Migrate to `rinja` (`rinja-rs/rinja`, ~1,000 stars, active fork of askama) or to `askama-rs/askama` ≥0.12.x once axum 0.8 compat confirmed. |
| `reqwest` | 0.12 | seanmonstar/reqwest | ~11,500 | Single individual maintainer (`seanmonstar`, `User` type — no org backing). 460 open issues. No SECURITY.md. High-risk feature set: TLS, HTTP, crypto (used in vault's proxy forwarding path). | No drop-in replacement; `hyper` (tokio-rs org, ~14k stars) if willing to accept lower-level API. Monitor for org transfer. |
| `security-framework` | 3.x | kornelski/rust-security-framework | ~294 | **Low popularity** (~294 stars). **Single active maintainer** (`kornelski`). No SECURITY.md. Prior CVE: RUSTSEC-2017-0003 / CVE-2017-18588 (MITM via skipped hostname verification; patched ≥0.1.12). macOS Keychain operations — high-sensitivity security surface. | `keyring` crate (~800 stars, multi-platform, multiple maintainers) as higher-level abstraction. `/usr/bin/security` CLI (already used in this codebase for ACL) could be extended to reduce FFI surface. |
| `rpassword` | 7.4.0 | conradkleinespel/rpassword | ~274 | **Low popularity** (~274 stars). **Single maintainer** (`conradkleinespel`). No SECURITY.md. Handles sensitive terminal password input directly. | `dialoguer` (~1,400 stars, multiple maintainers, actively maintained) for secure password prompts. |
| `sqlx` | 0.8.6 | launchbadge/sqlx | ~16,900 | **Known CVE** — RUSTSEC-2024-0363 / GHSA-xmrp-424f-vfpx: binary protocol misinterpretation / format injection via truncating casts (DEF CON 32, Aug 2024). Patched in ≥0.8.1. **Current version 0.8.6 is patched.** Flagged for record: 734 open issues, no SECURITY.md. | No migration needed (vulnerability patched). `Diesel` as alternative ORM. |
| `dirs` | *(transitive, not direct)* | dirs-dev/dirs-rs (**archived** Jan 2025) | ~734 | **Archived** — dirs-dev/dirs-rs and dirs-dev/directories-rs both archived Jan 2025. RUSTSEC-2020-0053 (unmaintained advisory, later withdrawn). Not a direct dependency in this workspace's Cargo.toml files but may appear as transitive dep. No SECURITY.md. | `etcetera` crate (cross-platform XDG + Apple dirs, actively maintained). Note: only relevant if a transitive dep pulls it in. |

---

## Preliminary Findings — Per-Dependency Severity

| Dependency | Severity | Rationale |
|---|---|---|
| `askama` / `askama_axum` | **HIGH** | Upstream archived; pinned versions on a dead release line; no future security patches |
| `reqwest` | **MEDIUM** | Single individual maintainer, high-risk feature set (TLS/HTTP), no SECURITY.md; no current CVEs |
| `security-framework` | **MEDIUM** | Low popularity, single maintainer, no SECURITY.md; historical CVE patched; sensitive Keychain surface |
| `rpassword` | **MEDIUM** | Low popularity, single maintainer, no SECURITY.md; handles terminal password input |
| `sqlx` | **LOW** | Historical CVE patched in current version (0.8.6 ≥ 0.8.1 requirement); no active vulnerability |
| `dirs` | **LOW** | Archived, but transitive only — not a direct dep; advisory withdrawn |
| `tokio` | **INFO** | tokio-rs org-backed, ~31,600 stars, active, no CVEs |
| `axum` | **INFO** | tokio-rs org-backed, ~25,600 stars, active, no CVEs |
| `blake3` | **INFO** | BLAKE3-team org-backed, ~6,100 stars, active, no CVEs |
| `serde` / `serde_json` | **INFO** | serde-rs org, ~10,500+ stars, active, no CVEs |
| `clap` | **INFO** | clap-rs org, ~16,300 stars, active, no CVEs |
| `chrono` | **INFO** | Historical RUSTSEC-2020-0159 patched ≥0.4.20; project uses 0.4.x (current) |
| `thiserror` / `anyhow` | **INFO** | dtolnay maintained, widely trusted, no CVEs |
| `tracing` / `tracing-subscriber` | **INFO** | tokio-rs org, active, no CVEs |
| `uuid` | **INFO** | uuid-rs, active, no CVEs |
| `async-trait` / `tower-http` | **INFO** | No risk signals |

---

## Counts by Risk Factor

| Risk Factor | Flagged Deps | Count |
|---|---|---|
| Archived / unmaintained upstream | `askama`/`askama_axum`, `dirs` | 2 |
| Single individual maintainer | `reqwest`, `security-framework`, `rpassword` | 3 |
| Low popularity (<500 stars) | `security-framework`, `rpassword` | 2 |
| High-risk feature set | `security-framework` (Keychain FFI), `reqwest` (TLS/HTTP) | 2 |
| Known CVE — patched | `sqlx` (RUSTSEC-2024-0363), `security-framework` (RUSTSEC-2017-0003) | 2 |
| No SECURITY.md / security contact | `sqlx`, `reqwest`, `security-framework`, `askama-rs/askama` | 4 |
| **Total flagged dependencies** | | **6** |

---

## Executive Summary

Six direct (or immediate transitive) dependencies carry at least one risk signal. No dependency has an **actively-exploited unpatched CVE** — the highest-severity finding is structural:

- **`askama`/`askama_axum`** are pinned to an archived upstream release line. The maintainer has migrated to `askama-rs/askama`, but the old versions will receive no security patches. The intentional pin for axum 0.8 compat (documented in CLAUDE.md) should be re-evaluated as part of a migration.
- **`reqwest`** carries meaningful supply-chain risk due to single-individual ownership of a high-value, high-use crate in the proxy forwarding path. No current exploit, but organizational structure is fragile.
- **`security-framework`** and **`rpassword`** are low-popularity, single-maintainer crates in security-sensitive roles (Keychain access and password prompting). The Keychain crate in particular operates at the macOS FFI boundary — a compromise here would directly expose stored credentials.

Low-risk baseline: `tokio`, `axum`, `blake3`, `serde`/`serde_json`, `clap`, `thiserror`, `anyhow`, `tracing`/`tracing-subscriber`, `uuid` — all org-backed or widely popular with clean advisory histories.

---

## Recommendations

1. **Migrate `askama`/`askama_axum`** — Plan migration to `askama-rs/askama` (or `rinja`) before the archived versions become an unpatched vulnerability liability. Re-evaluate the axum 0.8 compat pin as part of this work.

2. **Add `cargo audit`** to CI — The RustSec advisory DB (rustsec/advisory-db) would have surfaced RUSTSEC-2024-0363 automatically. A `cargo audit` step in CI catches future advisories at patch time.

3. **Replace `rpassword`** with `dialoguer` — Same functionality, more maintainers, higher popularity. Low migration effort.

4. **Monitor `reqwest`** — File an issue requesting a SECURITY.md. If the maintainer becomes unavailable, evaluate migrating the proxy forwarding path to `hyper` directly (tokio-rs org-backed).

5. **Evaluate `security-framework` exposure** — Extend the existing `/usr/bin/security` CLI approach (already used for ACL operations in this codebase) to reduce the FFI surface, or adopt `keyring` as a higher-level multi-maintainer abstraction.

6. **Confirm `dirs` is not a direct dep** — The project's `Cargo.toml` files do not list `dirs` as a direct dependency. Run `cargo tree | grep dirs` to confirm whether it appears as a transitive dep and from which crate.

---

*Scan completed: 2026-04-14. All star counts, issue counts, and repository states verified via GitHub API on this date.*
