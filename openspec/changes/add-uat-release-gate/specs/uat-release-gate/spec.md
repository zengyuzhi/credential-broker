## ADDED Requirements

### Requirement: UAT source-of-truth document SHALL exist at `docs/UAT.md`

The repository SHALL contain a single file `docs/UAT.md` that serves as the source of truth for user-acceptance-test entries. The file SHALL contain: a preamble explaining UAT's purpose, a tag-taxonomy section, a `## Running as an AI agent` guideline section, UAT entries grouped by capability area, a persona-journey section, and a run-log template appendix.

#### Scenario: New contributor discovers the UAT contract from one file

- **WHEN** a contributor (human or AI agent) encounters this repository for the first time
- **THEN** reading `docs/UAT.md` alone tells them the full UAT surface, entry format, pass criteria, and how to run it
- **AND** no parallel `docs/uat/`, `scripts/uat-*.md`, or `~/.claude/skills/uat-*.md` file is required to understand the contract

#### Scenario: The UAT doc is grep-friendly

- **WHEN** a script or AI agent runs `grep '^#### UAT-' docs/UAT.md`
- **THEN** the output lists every UAT entry identifier in the doc exactly once
- **AND** no line starting with `#### UAT-` appears outside an actual entry heading

### Requirement: UAT entries SHALL use a fixed five-field structured format

Each UAT entry in `docs/UAT.md` SHALL begin with a markdown `#### UAT-<area>-<NNN>` heading where `<area>` is a short kebab/lower-case label (e.g. `cli`, `dash`, `serve`, `proxy-oai`, `migrate`) and `<NNN>` is a zero-padded decimal. The heading SHALL be followed by a bullet block containing exactly these five fields in this order: `**Type:**`, `**Cap:**`, `**Cmd:**` (for `[AUTO:*]`) or `**Steps:**` (for `[MANUAL:*]`), `**Pass:**`, `**AI-safe:**`. Additional optional fields (`**Preconditions:**`, `**EstCost:**`, `**Evidence-scrub:**`) MAY appear after `**Cap:**` when relevant.

#### Scenario: AUTO entry parses cleanly

- **WHEN** the entry is `[AUTO:ANY]`
- **THEN** `Cmd:` is a single shell-runnable command
- **AND** `Pass:` is a regex (including the degenerate form `exit 0`) that deterministically matches successful stdout or exit behavior

#### Scenario: MANUAL entry parses cleanly

- **WHEN** the entry is `[MANUAL:USER]` or `[MANUAL:SHELL]`
- **THEN** `Steps:` is an ordered list enumerating human actions
- **AND** `Pass:` is free-text criteria the human can evaluate ("dashboard row flips within 4 seconds")

#### Scenario: Heading IDs are globally unique

- **WHEN** the doc is validated
- **THEN** no two `#### UAT-<area>-<NNN>` headings share the same identifier

### Requirement: Four type tags SHALL dispatch UAT entries to the correct runner

UAT entries SHALL carry exactly one of four type tags in the `**Type:**` field: `[AUTO:ANY]`, `[AUTO:CI]`, `[MANUAL:SHELL]`, `[MANUAL:USER]`. Each tag SHALL have a documented meaning in `docs/UAT.md`: `[AUTO:ANY]` is safe for an AI agent tool call, a shell script, or a human; `[AUTO:CI]` is safe for a shell/CI script but not an AI agent (multi-step state spanning tool calls, background processes, file-system writes that must be cleaned); `[MANUAL:SHELL]` requires the human's keyboard (keychain prompt, stdin password); `[MANUAL:USER]` requires the human's eyes (browser, visual diff, live updates).

#### Scenario: AI agent refuses [AUTO:CI] without shell-script fallback

- **WHEN** an AI agent encounters a `[AUTO:CI]` entry during a UAT run
- **THEN** the agent records the entry as `SKIP` with reason "AI-unsafe, defer to shell runner"
- **AND** the agent does not attempt to execute the `Cmd:` directly

#### Scenario: AI agent refuses [MANUAL:*] entries

- **WHEN** an AI agent encounters a `[MANUAL:SHELL]` or `[MANUAL:USER]` entry
- **THEN** the agent prints the entry's `Steps:` to the user and asks for a PASS/FAIL report
- **AND** the agent does not claim PASS without explicit user confirmation

### Requirement: UAT run-logs SHALL be stored per-release in `docs/uat-runs/`

Every UAT run SHALL produce exactly one file at `docs/uat-runs/<YYYY-MM-DD>-<version>[-<suffix>].md`. The file SHALL begin with a YAML front-matter block listing `version`, `date`, `runner` (human name or AI model ID), and `commit_sha`. The file body SHALL contain a summary line (`Total | Pass | Fail | Skip`), an explicit `Golden path: N/4` counter, a final `Gate: PASS|FAIL` verdict, a results table keyed by UAT-ID with `Type | Result | Evidence` columns, and a `## Failures` section detailing every `FAIL` row with next-action text.

#### Scenario: A run-log captures enough evidence for later review

- **WHEN** someone reads a run-log six months later
- **THEN** they can identify which release was gated, what pass/fail breakdown justified the gate decision, and which specific failures (if any) required follow-up
- **AND** no secret material (raw API key, raw session token, PIN) appears anywhere in the file

#### Scenario: Run-log for a docs-only release records the skip

- **WHEN** `git diff <last-tag>..HEAD` touches only `*.md` files
- **THEN** the run-log body may be a single line: `UAT skipped: docs-only diff since <prior-run-path>`
- **AND** the skip phrasing follows the exact template so release retrospectives can grep for it

### Requirement: UAT release gate SHALL block tags on golden-path or threshold failure

The release-process UAT step SHALL compute a PASS/FAIL verdict from a run-log using: (a) 4/4 golden-path entries (`UAT-CLI-001` through `UAT-CLI-004`) MUST report `PASS`; (b) `[AUTO:ANY]` entries MUST reach ≥95% pass rate across all non-SKIP results; (c) `[MANUAL:USER]` entries MUST reach ≥80% pass rate across all non-SKIP results; (d) `[AUTO:CI]` and `[MANUAL:SHELL]` results SHALL be informational only and SHALL NOT block the gate; (e) entries marked `SKIP` with documented reason SHALL NOT count toward denominators.

#### Scenario: Any golden-path failure blocks the tag

- **WHEN** the run-log shows `UAT-CLI-003` as `FAIL`
- **THEN** the release-process UAT step reports `Gate: FAIL` even if all other entries pass
- **AND** the release cannot proceed to tag push until the golden-path failure is resolved or the UAT entry is re-disposed via spec update

#### Scenario: Single flaky AUTO:ANY failure is tolerated

- **WHEN** 20 `[AUTO:ANY]` entries ran, 19 passed, 1 failed → 95% pass rate
- **THEN** the gate reports `PASS` on the AUTO:ANY dimension
- **AND** the failure is noted in the run-log's `## Failures` section for next-release triage

#### Scenario: Documented skip does not harm pass rate

- **WHEN** `UAT-PROXY-OAI-001` is skipped because `$UAT_ALLOW_PAID` is unset, with that reason recorded
- **THEN** the entry is excluded from both numerator and denominator of the AUTO:ANY pass-rate calculation

### Requirement: UAT doc SHALL carry an AI-agent runner guideline

`docs/UAT.md` SHALL contain a section titled `## Running as an AI agent` that, at minimum: (a) enumerates the ordered steps an AI agent SHALL follow when running UAT (read doc → execute `[AUTO:ANY]` → skip `[AUTO:CI]` → prompt user for `[MANUAL:*]` → write run-log → report gate); (b) lists forbidden actions (claiming PASS without running, skipping MANUAL silently, inventing UAT-IDs, running paid entries without explicit consent, modifying the UAT doc during a run); (c) specifies the run-log file-naming convention with `-ai` suffix when the runner is an AI agent.

#### Scenario: AI agent reads the doc and knows what to do

- **WHEN** a fresh Claude session receives `/uat-run` (or the user says "run UAT")
- **THEN** the session reads `docs/UAT.md`, locates the `## Running as an AI agent` section, and follows its ordered steps without re-deriving the workflow
- **AND** the session produces a run-log at `docs/uat-runs/<date>-<version>-ai.md` conforming to the file-format requirement above

#### Scenario: AI agent refuses to burn money silently

- **WHEN** an entry carries `[MANUAL:USER]` and is gated on `$UAT_ALLOW_PAID=1`
- **THEN** the AI agent does not set the env var on its own initiative
- **AND** the agent requires explicit in-session user consent ("yes, run paid UAT") before dispatching such an entry to the user

### Requirement: Paid-provider UAT entries SHALL be gated behind `$UAT_ALLOW_PAID`

Any UAT entry whose `Cmd:` or `Steps:` invokes a real provider API (OpenAI, Anthropic, TwitterAPI, etc. — any call that consumes the author's paid quota) SHALL declare `**EstCost:**` and SHALL gate execution on `$UAT_ALLOW_PAID=1`. Unless the variable is set, the entry result SHALL be `SKIP` with reason `$UAT_ALLOW_PAID not set`.

#### Scenario: Paid entry skipped by default

- **WHEN** a UAT run is invoked without `$UAT_ALLOW_PAID=1`
- **THEN** every entry carrying an `EstCost:` field resolves to `SKIP`
- **AND** the run-log records the reason string `$UAT_ALLOW_PAID not set`

#### Scenario: Paid entry runs with explicit opt-in

- **WHEN** the human maintainer exports `UAT_ALLOW_PAID=1` and re-runs
- **THEN** paid entries execute normally and contribute to their type-tag pass rate

### Requirement: First-pass UAT doc SHALL cover the 14 canonical capabilities plus 4 golden-path flows

The initial `docs/UAT.md` committed with this change SHALL contain at least one UAT entry per canonical capability in `openspec/specs/` (14 capabilities as of 2026-04-14) PLUS exactly four golden-path entries `UAT-CLI-001` through `UAT-CLI-004` covering the flows `vault credential add`, `vault profile bind`, `vault run --profile <name> -- <cmd>`, `vault stats --json`. Total entry count SHALL be between 20 and 35 inclusive.

#### Scenario: Coverage audit passes

- **WHEN** a reviewer compares the `**Cap:**` fields of all UAT entries against `ls openspec/specs/`
- **THEN** every canonical capability name appears in at least one entry's `Cap:` field
- **AND** no orphan `Cap:` value references a non-existent capability

#### Scenario: Golden-path IDs are reserved

- **WHEN** the doc is validated
- **THEN** `UAT-CLI-001`, `UAT-CLI-002`, `UAT-CLI-003`, `UAT-CLI-004` map (in any order) to the four flows listed above
- **AND** no other UAT ID uses the `UAT-CLI-00{1,2,3,4}` identifiers for different flows

### Requirement: Retiring a canonical capability SHALL retire its UAT entries

When an OpenSpec change removes or renames a canonical capability under `openspec/specs/`, the same change SHALL update or remove every UAT entry in `docs/UAT.md` whose `**Cap:**` field references that capability. No commit SHALL leave `docs/UAT.md` referencing a non-existent `**Cap:**`.

#### Scenario: Capability removal updates UAT in the same commit

- **WHEN** a future change removes capability `vault-ui-auto-start`
- **THEN** the same change deletes every UAT entry with `**Cap:** vault-ui-auto-start`
- **AND** the change's tasks.md has an explicit task for the UAT update
