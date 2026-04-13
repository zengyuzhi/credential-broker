## MODIFIED Requirements

### Requirement: vault ui auto-starts daemon
The `vault ui` command SHALL automatically start the daemon in the background if the `/health` probe fails, instead of bailing with an error.

#### Scenario: Daemon not running
- **WHEN** user runs `vault ui` and no server is running on port 8765
- **THEN** the CLI starts the daemon in the background (equivalent to `vault serve --background`)
- **AND** waits for `/health` to succeed (up to 5 seconds)
- **AND** then proceeds with challenge request and browser open as normal
- **AND** prints "Started vault server in background (pid: XXXX)"

#### Scenario: Daemon already running
- **WHEN** user runs `vault ui` and the server is already running
- **THEN** behavior is unchanged (request challenge, open browser)

#### Scenario: Daemon fails to start
- **WHEN** user runs `vault ui` and the background daemon fails to start within 5 seconds
- **THEN** the CLI fails with "Could not start vault server. Check port 8765 or run vault serve manually."
