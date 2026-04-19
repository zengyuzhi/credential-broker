# Repository Guidelines

## Project Structure & Module Organization
`credential-broker` is a Rust workspace. Core code lives in `crates/`: `vault-cli` is the user-facing binary, `vaultd` serves the local dashboard/proxy, and the remaining crates handle domain types, SQLite persistence, Keychain access, provider adapters, policy, and telemetry. Integration tests live under `crates/*/tests`. Supporting material sits in `docs/`, schema changes in `migrations/`, runnable examples in `examples/`, and release/spec work in `openspec/`.

## Build, Test, and Development Commands
- `cargo build -p vault-cli`: build the main `vault` binary.
- `cargo test --workspace`: run the full test suite used by CI.
- `cargo clippy --workspace --all-targets -- -D warnings`: enforce lint cleanliness.
- `cargo fmt --all`: format the workspace.
- `cargo run -p vault-cli -- --help`: inspect CLI entry points locally.
- `cargo run -p vault-cli -- serve`: start the loopback-only server for dashboard/proxy work.

## Coding Style & Naming Conventions
Use stable Rust with `rustfmt` defaults (4-space indentation, trailing commas where formatter adds them). Keep modules focused and prefer explicit names such as `issue_lease`, `store_smoke`, or `serve_state_path`. Follow existing crate naming (`vault-*`) and keep SQL migrations ordered numerically (`0001_init.sql`). Avoid logging secrets, raw tokens, or Keychain material.

## Testing Guidelines
Add unit tests next to the code they cover when practical, and place cross-crate or CLI flows in `crates/<crate>/tests/*.rs`. Favor scenario-driven integration test names like `upgrade_local_fixtures.rs`. Before opening a PR, run `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo fmt --all -- --check`.

## Commit & Pull Request Guidelines
Recent history uses Conventional Commit prefixes: `feat(...)`, `fix(...)`, `docs(...)`, `chore(...)`, `style(...)`. Keep subjects imperative and scoped, for example `fix(vault-cli): use user-owned state dir`. PRs should summarize behavior changes, list verification commands, and call out any updates to `CHANGELOG.md`, `docs/UAT.md`, or `docs/ARCHITECTURE.md`. Include screenshots only when dashboard/UI output changes.

## Security & Configuration Tips
This project is macOS-first and intentionally keeps secrets out of files. Never commit `.env` secrets, sample API keys, or copied Keychain values. Prefer test fixtures and synthetic release assets over real credentials when working on upgrade, proxy, or telemetry flows.

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **credential-broker** (2507 symbols, 4128 relationships, 211 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> If any GitNexus tool warns the index is stale, run `npx gitnexus analyze` in terminal first.

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `gitnexus_impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `gitnexus_detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `gitnexus_query({query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `gitnexus_context({name: "symbolName"})`.

## When Debugging

1. `gitnexus_query({query: "<error or symptom>"})` — find execution flows related to the issue
2. `gitnexus_context({name: "<suspect function>"})` — see all callers, callees, and process participation
3. `READ gitnexus://repo/credential-broker/process/{processName}` — trace the full execution flow step by step
4. For regressions: `gitnexus_detect_changes({scope: "compare", base_ref: "main"})` — see what your branch changed

## When Refactoring

- **Renaming**: MUST use `gitnexus_rename({symbol_name: "old", new_name: "new", dry_run: true})` first. Review the preview — graph edits are safe, text_search edits need manual review. Then run with `dry_run: false`.
- **Extracting/Splitting**: MUST run `gitnexus_context({name: "target"})` to see all incoming/outgoing refs, then `gitnexus_impact({target: "target", direction: "upstream"})` to find all external callers before moving code.
- After any refactor: run `gitnexus_detect_changes({scope: "all"})` to verify only expected files changed.

## Never Do

- NEVER edit a function, class, or method without first running `gitnexus_impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `gitnexus_rename` which understands the call graph.
- NEVER commit changes without running `gitnexus_detect_changes()` to check affected scope.

## Tools Quick Reference

| Tool | When to use | Command |
|------|-------------|---------|
| `query` | Find code by concept | `gitnexus_query({query: "auth validation"})` |
| `context` | 360-degree view of one symbol | `gitnexus_context({name: "validateUser"})` |
| `impact` | Blast radius before editing | `gitnexus_impact({target: "X", direction: "upstream"})` |
| `detect_changes` | Pre-commit scope check | `gitnexus_detect_changes({scope: "staged"})` |
| `rename` | Safe multi-file rename | `gitnexus_rename({symbol_name: "old", new_name: "new", dry_run: true})` |
| `cypher` | Custom graph queries | `gitnexus_cypher({query: "MATCH ..."})` |

## Impact Risk Levels

| Depth | Meaning | Action |
|-------|---------|--------|
| d=1 | WILL BREAK — direct callers/importers | MUST update these |
| d=2 | LIKELY AFFECTED — indirect deps | Should test |
| d=3 | MAY NEED TESTING — transitive | Test if critical path |

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/credential-broker/context` | Codebase overview, check index freshness |
| `gitnexus://repo/credential-broker/clusters` | All functional areas |
| `gitnexus://repo/credential-broker/processes` | All execution flows |
| `gitnexus://repo/credential-broker/process/{name}` | Step-by-step execution trace |

## Self-Check Before Finishing

Before completing any code modification task, verify:
1. `gitnexus_impact` was run for all modified symbols
2. No HIGH/CRITICAL risk warnings were ignored
3. `gitnexus_detect_changes()` confirms changes match expected scope
4. All d=1 (WILL BREAK) dependents were updated

## Keeping the Index Fresh

After committing code changes, the GitNexus index becomes stale. Re-run analyze to update it:

```bash
npx gitnexus analyze
```

If the index previously included embeddings, preserve them by adding `--embeddings`:

```bash
npx gitnexus analyze --embeddings
```

To check whether embeddings exist, inspect `.gitnexus/meta.json` — the `stats.embeddings` field shows the count (0 means no embeddings). **Running analyze without `--embeddings` will delete any previously generated embeddings.**

> Claude Code users: A PostToolUse hook handles this automatically after `git commit` and `git merge`.

## CLI

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->
