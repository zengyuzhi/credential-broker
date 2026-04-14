# Security Audits

Dated security-audit baselines for credential-broker. Each baseline is a
self-contained snapshot — raw scanner output plus a consolidated SUMMARY
with per-finding severity and disposition.

## Latest baseline

**[`2026-04-14-tob-baseline/`](./2026-04-14-tob-baseline/)** — first Trail of Bits audit pass
(zeroize-audit, supply-chain-risk-auditor, sharp-edges) against the v0.1.0
codebase.

## Directory convention

```
docs/audits/
├── README.md                       (this file — points at the latest baseline)
└── YYYY-MM-DD-<slug>/              (one directory per baseline; dated)
    ├── SUMMARY.md                  (consolidated, human-facing)
    ├── <skill-name>.md             (one file per skill invoked)
    └── …
```

Older baselines are preserved as history — they are never deleted, only
superseded. To compare releases, diff two SUMMARY.md files.

## Severity rubric (applied in SUMMARY.md)

| Level       | Meaning                                                                                            |
|-------------|----------------------------------------------------------------------------------------------------|
| **CRITICAL** | RCE, credential theft, auth bypass, key leak to disk/log/network. Ship-stopper.                  |
| **HIGH**     | Privilege escalation, non-constant-time secret compare, missing zeroize on raw API keys, actively-exploited CVE in direct dep. |
| **MEDIUM**   | Defense-in-depth gap without active exploit, easy-to-hit sharp edge with bounded impact, moderate-risk dep. |
| **LOW**      | Style-level crypto hygiene, misleading comment/doc, minor footgun.                                |
| **INFO**     | Observation only; no action implied.                                                              |

## Disposition taxonomy

Every finding in a SUMMARY.md ends in exactly one terminal disposition:

- **Fix now** — code change lands in the same OpenSpec change as the baseline. Entry cites the commit short-SHA.
- **Triage** — copied to `docs/ROADMAP.md` with tag `(audit: <skill> YYYY-MM-DD)` and a complexity estimate.
- **Accept** — documented rationale; no code change. Entry includes a paragraph explaining why (false positive, scope-out, mitigated by other control, etc.).

No "pending" or "TBD" items are allowed in a committed SUMMARY. If a finding
can't be dispositioned inside the change's fix budget, re-dispose to
Triage before archive.

## Fix budget per change

A baseline-introducing change fixes **CRITICAL and HIGH findings only**.
Everything MEDIUM and below is triaged to ROADMAP or explicitly Accepted.
This keeps audit changes finite; deeper remediation work becomes its own
OpenSpec change with the ROADMAP bullet as the seed.

## Re-running

The release-readiness checklist in [`../RELEASE.md`](../RELEASE.md) invokes
a Security audit pass step before every tag. That step creates a new dated
directory and updates the "Latest baseline" pointer above in the same commit
that lands the baseline. The gate is **comparative** — a release is blocked
only if *new* CRITICAL or HIGH items appear relative to the prior baseline,
not if the baseline reports any CRITICAL or HIGH at all. This prevents a
single false positive from becoming a permanent release blocker.

## Invocation

The audit skills are Claude-interactive (no headless CI mode yet):

```
/plugin install zeroize-audit@trailofbits/skills
/plugin install supply-chain-risk-auditor@trailofbits/skills
/plugin install sharp-edges@trailofbits/skills
```

Then invoke each skill from a Claude Code session and save output to the
new dated baseline directory per the convention above.
