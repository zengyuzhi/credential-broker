## Context

The vault-cli binary uses clap's derive API. All structs (`Cli`, `CredentialCommand`, `RunCommand`, etc.) have `#[derive(Args)]` or `#[derive(Subcommand)]` but lack `about`, `long_about`, and `help` attributes. The result is functional but undocumented `--help` output.

The CLI has 5 command files:
- `main.rs` — root `Cli` struct with top-level subcommands
- `commands/credential.rs` — `add`, `list`, `enable`, `disable`, `remove`
- `commands/profile.rs` — `create`, `list`, `show`, `bind`
- `commands/run.rs` — `run --profile <name> -- <cmd>`
- `commands/stats.rs` — `stats [--provider <name>]`

## Goals / Non-Goals

**Goals:**
- Every `--help` output is self-explanatory without needing docs or source code
- Binary displays as `vault` (not `vault-cli`) in help text
- Argument descriptions explain purpose, defaults, and valid values

**Non-Goals:**
- Shell completions (future work)
- Man page generation
- Changing any command behavior or argument parsing
- Adding new commands or flags

## Decisions

**1. Use clap `about` attribute on every subcommand, `help` on every arg/flag**

Rationale: This is the standard clap pattern. Each `#[command(about = "...")]` adds a one-line description shown in parent help. Each `#[arg(help = "...")]` describes the argument inline.

Alternative considered: External help text file loaded at build time — rejected as over-engineering for a small CLI.

**2. Set binary display name via `#[command(name = "vault")]`**

Rationale: The binary is compiled as `vault-cli` but users think of it as `vault`. Setting `name` in clap changes only the help text display, not the binary filename.

**3. Keep descriptions terse — one line per item**

Rationale: CLI help should be scannable. Long prose belongs in README or docs, not `--help` output.

## Risks / Trade-offs

- [Risk] Help text gets stale as features evolve → Mitigation: text is co-located with the code it describes, so changes to args naturally prompt help updates.
- [Risk] Binary name mismatch (`vault` in help vs `vault-cli` on disk) → Mitigation: users can alias `vault` to `vault-cli`; a future install step can rename the binary.
