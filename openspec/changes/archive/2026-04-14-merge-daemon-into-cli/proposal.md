## Why

Using the vault dashboard currently requires two terminals: one for `cargo run -p vaultd` (daemon) and one for `vault ui` (open browser). Users must know to start the daemon first — if they forget, `vault ui` fails with a confusing connection error. This friction is unnecessary for a local developer tool.

Merging the daemon into the vault CLI as a `vault serve` subcommand, and making `vault ui` auto-start it when not running, creates a one-command experience.

## What Changes

- Add `vault serve` subcommand that starts the HTTP server (replaces `cargo run -p vaultd`)
- Add `vault serve --background` to fork the server into the background with PID file management
- Add `vault serve stop` to cleanly shut down a backgrounded server
- Modify `vault ui` to auto-start the daemon in the background if `/health` probe fails
- The `vaultd` crate becomes a library (no more separate binary), re-exported through vault-cli

## Out of Scope

- Removing the vaultd crate entirely (it stays as a library for code organization)
- Auto-shutdown on idle (future enhancement)
- systemd/launchd service integration

## Capabilities

### New Capabilities
- `vault-serve`: Start the HTTP dashboard and proxy server from the vault CLI
- `vault-serve-lifecycle`: Background process management with PID file tracking

### Modified Capabilities
- `vault-ui-auto-start`: `vault ui` auto-starts daemon when not running
