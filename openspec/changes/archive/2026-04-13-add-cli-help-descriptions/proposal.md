## Why

The vault-cli binary has working commands but all clap `about` and `help` attributes are empty. Running `vault --help` or `vault credential add --help` shows argument names but no descriptions, making the tool hard to use without reading source code. This is a quick polish pass before the first public release.

## What Changes

- Add `about` descriptions to the top-level CLI and every subcommand (`credential`, `profile`, `run`, `stats`)
- Add `help` text to every argument and flag across all subcommands
- Add `long_about` to the root command with a one-paragraph description
- Rename the binary from `vault-cli` to `vault` for cleaner UX (via clap `name` attribute)

## Capabilities

### New Capabilities

- `cli-help-text`: Comprehensive help descriptions for all vault-cli commands, subcommands, arguments, and flags

### Modified Capabilities

_(none — this is purely additive documentation, no behavior changes)_

## Impact

- **Affected crate:** `vault-cli` only (no other crates touched)
- **Files:** `main.rs`, `commands/credential.rs`, `commands/profile.rs`, `commands/run.rs`, `commands/stats.rs`
- **No API changes, no behavior changes, no new dependencies**
- **Testing:** existing tests remain valid — this only adds clap metadata attributes
