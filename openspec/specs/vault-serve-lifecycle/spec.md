## ADDED Requirements

### Requirement: Background mode with PID file
The `vault serve` command SHALL accept a `--background` flag that forks the server into the background and writes a PID file to `.local/vault.pid`. The command SHALL exit after confirming the server is healthy.

#### Scenario: Background start
- **WHEN** user runs `vault serve --background`
- **THEN** the server starts as a detached background process
- **AND** a PID file is written to `.local/vault.pid`
- **AND** the CLI waits for `/health` to return 200 before exiting
- **AND** the CLI prints "Vault server started (pid: XXXX)"

#### Scenario: Already running
- **WHEN** user runs `vault serve --background` and a server is already running on the port
- **THEN** the command prints "Vault server is already running (pid: XXXX)" and exits successfully

### Requirement: Graceful shutdown via vault serve stop
The CLI SHALL provide `vault serve stop` that reads the PID file and sends SIGTERM to the daemon process.

#### Scenario: Stop running server
- **WHEN** user runs `vault serve stop` and a daemon is running
- **THEN** the PID file is read, SIGTERM is sent, PID file is removed
- **AND** the CLI prints "Vault server stopped"

#### Scenario: Stop when not running
- **WHEN** user runs `vault serve stop` and no PID file exists
- **THEN** the CLI prints "Vault server is not running"

### Requirement: vault serve status
The CLI SHALL provide `vault serve status` that checks whether the server is running.

#### Scenario: Status when running
- **WHEN** user runs `vault serve status` and the server is healthy
- **THEN** the CLI prints "Vault server is running (pid: XXXX, port: 8765)"

#### Scenario: Status when stopped
- **WHEN** user runs `vault serve status` and no server is running
- **THEN** the CLI prints "Vault server is not running"
