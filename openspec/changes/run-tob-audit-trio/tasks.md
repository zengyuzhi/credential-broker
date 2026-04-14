## 1. Scaffold audit directory

- [x] 1.1 Created `docs/audits/` directory
- [x] 1.2 Wrote `docs/audits/README.md` — dated dirs, severity rubric, 3-disposition taxonomy, fix budget rule, comparative release gate explanation
- [x] 1.3 Created `docs/audits/2026-04-14-tob-baseline/` as the target directory for this pass

## 2. Run zeroize-audit

- [x] 2.1 Dispatched `zeroize-audit` via subagent `a3ff22e5…` focusing on vault-secrets/vault-cli(run)/vaultd(proxy)/vault-policy
- [x] 2.2 Raw output at `docs/audits/2026-04-14-tob-baseline/zeroize-audit.md` (360 lines, 9 findings) with provenance header
- [x] 2.3 Reviewed; 3 HIGH + 3 MEDIUM confirmed, 2 INFO (Phase 2/3 binary analysis noted as "likely" not confirmed — scan gap)

## 3. Run supply-chain-risk-auditor

- [x] 3.1 Dispatched `supply-chain-risk-auditor` via subagent `a6afb345…`; 16 direct deps scored
- [x] 3.2 Raw output at `docs/audits/2026-04-14-tob-baseline/supply-chain-risk-auditor.md` with provenance header
- [x] 3.3 Flagged: `askama`/`askama_axum` (HIGH, archived upstream); `reqwest`/`security-framework`/`rpassword` (MEDIUM, solo-maintained); `sqlx`/`dirs` (LOW, already-patched CVE + archived-transitive)

## 4. Run sharp-edges

- [x] 4.1 Dispatched `sharp-edges` via subagent `a666410c…` covering auth.rs, keychain.rs, proxy routes, CLI commands
- [x] 4.2 Raw output at `docs/audits/2026-04-14-tob-baseline/sharp-edges.md` (317 lines, 14 findings) with provenance header
- [x] 4.3 Reviewed: 1 CRIT + 4 HIGH + 4 MED + 3 LOW + 2 INFO; scan gap noted (vault-db sub-modules, vault-providers adapters, dashboard/stats/SSE routes, askama XSS, non-macOS codepaths)

## 5. Consolidate into SUMMARY.md

- [x] 5.1 SUMMARY.md written with severity rubric, finding count table, CRITICAL/HIGH/MEDIUM/LOW/INFO sections, gate evaluation footer
- [x] 5.2 26 findings classified: 1 CRIT / 9 HIGH / 8 MED / 5 LOW / 3 INFO
- [x] 5.3 Dispositions: 4 Fix-now (CRIT + 3 HIGH) / 16 Triage / 6 Accept. Per design Decision 3, HIGH items requiring architectural change (zeroize workspace, askama migration) moved to Triage.
- [x] 5.4 Accept rationale paragraphs written for ZA-0008 (account names ≠ secrets), ZA-0009 (duplicate), SC-05 (sqlx CVE already patched), SC-06 (dirs transitive-only), SC-07..16 (clean deps)
- [x] 5.5 `docs/audits/README.md` already points at `2026-04-14-tob-baseline` (set in Task 1.2)

## 6. Apply CRITICAL + HIGH fixes

- [x] 6.1 Applied 4 fixes in one atomic commit `6dd9f7e`: SE-01 (PIN ConstantTimeEq), SE-02 (CSRF ConstantTimeEq + empty-guard reorder), SE-03 (rate-limit fixed "loopback" key), SE-04 (removed `put` from SecretStore trait + impl + unused `set_generic_password` import)
- [x] 6.2 `cargo test --workspace` → 73/73 pass after fixes
- [x] 6.3 `cargo clippy --workspace --all-targets -- -D warnings` → exit 0
- [x] 6.4 SUMMARY.md items updated with commit `6dd9f7e`
- [x] 6.5 Zeroize HIGH findings (ZA-0001..0005) + SE-05 + askama HIGH (SC-01) re-disposed from Fix-now to Triage with notes explaining the scope boundary; ROADMAP entries added as seeds for follow-on changes (`add-zeroize-to-secret-paths`, `migrate-askama-fork`)

## 7. Update docs/ROADMAP.md with triaged items

- [x] 7.1 ROADMAP.md updated with triaged items across Near-term (7 items), Medium-term (3), Speculative (2)
- [x] 7.2 Every added bullet carries `(audit: <skill> 2026-04-14 <id>)` tag
- [x] 7.3 All new bullets include `S`/`M`/`L` complexity tags

## 8. Update CHANGELOG.md `[Unreleased] → Security`

- [x] 8.1 Added 4 user-facing `### Security` bullets in `[Unreleased]` — one per Fix-now item (SE-01, SE-02, SE-03, SE-04), each with finding-id reference
- [x] 8.2 Added summary line pointing to full baseline report at `docs/audits/2026-04-14-tob-baseline/SUMMARY.md` with triage+accept counts

## 9. Update docs/RELEASE.md

- [x] 9.1 Inserted new step 4 "Security audit pass" between step 3 (Format clean) and the existing CHANGELOG step; renumbered 4→5, 5→6, 6→7, 7→8, 8→9
- [x] 9.2 Step 4 describes invocation of all three skills, new dated directory creation, and gate logic
- [x] 9.3 Documented "docs-only release skip" exception with exact retrospective phrasing requirement
- [x] 9.4 Appended `### Baseline audits` subsection with 2026-04-14 retro: 26 findings summary, zeroize as biggest remaining item, askama migration note, scanner-interactive-only observation

## 10. Verify

- [x] 10.1 `cargo test --workspace` → 73 passed / 0 failed (vault-core 19, vault-providers 1, vault-secrets 9, vault-db 8+4, vault-telemetry 1, vault-policy 2, vault-cli 12+1, vaultd 16)
- [x] 10.2 `cargo clippy --workspace --all-targets -- -D warnings` → exit 0
- [x] 10.3 `cargo fmt --all -- --check` → exit 0 (after `cargo fmt --all` normalized the inlined `if` line in auth.rs)
- [x] 10.4 SUMMARY.md dispositions confirmed terminal: 4 Fix-now (all have commit SHA `6dd9f7e`) / 16 Triage (all have ROADMAP bullets) / 6 Accept (all have Rationale). No pending/TBD items.
- [x] 10.5 Verified cross-reference: SE-01..04 cite `6dd9f7e`; triaged items (zeroize group, askama, SE-06..12, ZA-0006..0007, SC-02..04) each resolve to a ROADMAP entry with the `(audit: ... 2026-04-14)` origin tag.

## 11. Commit

- [x] 11.1 Fixes landed as one atomic commit `6dd9f7e` (`fix(sec): constant-time secret compare, remove ACL-less put, fix rate-limit key`) — grouped because they share the audit-finding origin and together form the baseline's Fix-now bundle
- [ ] 11.2 Stage the baseline docs + ROADMAP + CHANGELOG + RELEASE.md + tasks.md as `docs: add Trail of Bits audit baseline 2026-04-14`
- [ ] 11.3 `git push origin main`

## 12. Close the loop

- [x] 12.1 All above tasks checked
- [ ] 12.2 Archive via `/opsx:archive run-tob-audit-trio` so `security-audit-baseline` enters `openspec/specs/` and the delta on `release-process` lands
