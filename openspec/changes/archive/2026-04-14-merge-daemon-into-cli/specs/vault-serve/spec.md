## ADDED Requirements

### Requirement: vault serve starts the HTTP server
The CLI SHALL provide a `vault serve` subcommand that starts the vaultd HTTP server on `127.0.0.1:8765`. The server SHALL serve all existing routes (health, proxy, auth, dashboard pages, SSE). The command SHALL block until interrupted (Ctrl+C) unless `--background` is specified.

#### Scenario: Foreground serve
- **WHEN** user runs `vault serve`
- **THEN** the HTTP server starts on `127.0.0.1:8765`
- **AND** the terminal shows "Vault server listening on http://127.0.0.1:8765"
- **AND** the process blocks until Ctrl+C

#### Scenario: Custom port
- **WHEN** user runs `vault serve --port 9000`
- **THEN** the HTTP server starts on `127.0.0.1:9000`

#### Scenario: Port already in use
- **WHEN** user runs `vault serve` but port 8765 is already bound
- **THEN** the command fails with "Port 8765 is already in use. Is vault serve already running?"
