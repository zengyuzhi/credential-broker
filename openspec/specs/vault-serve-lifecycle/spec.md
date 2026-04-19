## ADDED Requirements

### Requirement: Background mode with PID file
The `vault serve` command SHALL accept a `--background` flag that forks the server into the background and writes a PID file to the **absolute** path returned by `vault_cli::support::config::state_dir().join("vault.pid")`. The state directory resolution MUST be cwd-independent: it derives from the filesystem path portion of the resolved `current_database_url()`, never from `std::env::current_dir()`. `vault serve stop`, `vault serve status`, `vault ui`, and `vault upgrade` MUST all consult this same absolute path. Because pre-migration PID files were written relative to arbitrary working directories, legacy handling is only best-effort: implementations MAY inspect a `.local/vault.pid` in the caller's current working directory for cleanup or same-directory compatibility, but MUST NOT claim arbitrary historical cwd-relative PID files are globally discoverable.

#### Scenario: Background start
- **WHEN** user runs `vault serve --background`
- **THEN** the server starts as a detached background process
- **AND** a PID file is written to `.local/vault.pid`
- **AND** the CLI waits for `/health` to return 200 before exiting
- **AND** the CLI prints "Vault server started (pid: XXXX)"

#### Scenario: Already running
- **WHEN** user runs `vault serve --background` and a server is already running on the port
- **THEN** the command prints "Vault server is already running (pid: XXXX)" and exits successfully

#### Scenario: PID file lives at an absolute, cwd-independent path
- **WHEN** a user runs `vault serve --background` from `/tmp/foo` AND later runs `vault serve stop` or `vault upgrade` from `$HOME/projects/bar`
- **THEN** both invocations resolve the PID file to the same absolute path under `state_dir()`
- **AND** `vault serve stop` successfully signals the running daemon regardless of the working directory of either invocation

#### Scenario: Legacy same-cwd `.local/vault.pid` is handled best-effort
- **WHEN** `vault serve status`, `vault serve stop`, or `vault upgrade` runs in a directory that still contains a legacy `.local/vault.pid` from a pre-migration CLI
- **THEN** the implementation MAY inspect that file for stale-file cleanup or same-directory compatibility when the canonical `state_dir()/vault.pid` is absent
- **AND** all newly written PID files go exclusively through `state_dir().join("vault.pid")`
- **AND** the docs do not claim that arbitrary old cwd-relative PID files are discoverable from unrelated directories

#### Scenario: State directory is created with owner-only permissions
- **WHEN** `state_dir()` is resolved for the first time
- **THEN** the directory is created with mode `0700` if it does not already exist
- **AND** subsequent PID writes use mode `0600`

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
