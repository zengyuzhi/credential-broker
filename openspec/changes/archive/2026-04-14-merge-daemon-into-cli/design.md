## Context

The vault workspace has two binaries: `vault-cli` (the CLI tool) and `vaultd` (the HTTP daemon). They share the same SQLite database but are separate processes. Users must manually start `vaultd` before using `vault ui` or the proxy feature, which is a poor developer experience.

The goal is to make `vault serve` a subcommand of `vault` so everything runs from a single binary, and `vault ui` auto-starts the daemon when needed.

## Goals / Non-Goals

**Goals:**
- Single binary that serves both CLI commands and the HTTP daemon
- `vault ui` works without manually starting the daemon first
- Clean background process management (start, stop, status)
- Preserve all existing `vaultd` functionality unchanged

**Non-Goals:**
- Removing the `vaultd` crate (it stays as a library, just loses its `main.rs`)
- Auto-shutdown on idle timeout (future enhancement)
- Running as a system service (launchd/systemd)

## Decisions

### 1. Convert vaultd from binary to library crate

Rationale: The vaultd crate currently has `src/main.rs` with a thin main function. We extract the server startup logic into a public `pub async fn start_server(database_url: &str, port: u16) -> anyhow::Result<()>` function in `vaultd/src/lib.rs`. The vault-cli crate depends on vaultd and calls this function from the `vault serve` handler.

The existing `vaultd/src/main.rs` can be removed entirely, or kept as a thin wrapper that calls the library function (for `cargo run -p vaultd` backwards compat during transition).

Alternative considered: Moving all vaultd code into vault-cli — rejected because it would make vault-cli massive and break the clean crate separation.

### 2. Background process via std::process::Command self-re-exec

Rationale: `vault serve --background` spawns a new process running `vault serve` (without `--background`) using `std::process::Command`. The child is detached via platform-specific APIs (`setsid` on Unix). The parent writes the child PID to `.local/vault.pid` and exits after confirming `/health` responds.

This is simpler than forking (no `unsafe` needed) and works cross-platform. The PID file lives alongside the database in `.local/`.

Alternative considered: `fork()` via nix crate — rejected because it requires `unsafe` and doesn't work on non-Unix. Thread-based — rejected because `vault serve` blocks the process, preventing other commands from running.

### 3. PID file at `.local/vault.pid`

Rationale: Co-located with the database file (`.local/vault.db`). The PID file contains just the process ID as a plain integer. On `vault serve stop`, read the PID, send SIGTERM (or `taskkill` on Windows), then remove the file. On `vault serve status`, read PID and check if process exists.

Stale PID detection: if the PID file exists but the process is dead, treat it as "not running" and clean up the stale file.

### 4. vault ui auto-start uses the same background spawn logic

Rationale: When `vault ui` fails the `/health` check, it calls the same `spawn_background_server()` function that `vault serve --background` uses. This keeps the behavior identical — same PID file, same health check wait, same error messages.

The health check retry loop waits up to 5 seconds (polling every 200ms) before giving up.

### 5. vault-cli gains vaultd dependencies

Rationale: Adding `vaultd` as a dependency of vault-cli pulls in axum, askama, tower-http, etc. This increases the binary size by ~2-3MB. Acceptable for a local dev tool — single-binary simplicity is worth more than a small binary.

## Risks / Trade-offs

- [Risk] Binary size increases ~2-3MB → Acceptable for a local tool
- [Risk] Background process could orphan if `vault serve stop` is never called → PID file + stale detection mitigates this
- [Risk] Port conflicts if user runs multiple instances → Port-in-use detection with clear error message
