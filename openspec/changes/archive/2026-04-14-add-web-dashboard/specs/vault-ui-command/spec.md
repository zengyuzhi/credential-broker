## MODIFIED Requirements

### Requirement: vault ui command
The CLI SHALL provide a `vault ui` subcommand that requests a challenge from vaultd via `POST /api/auth/challenge`, receives the PIN and challenge ID, prints the PIN to the terminal, and opens the dashboard in the default browser with the challenge ID in the URL.

#### Scenario: Launching the dashboard
- **WHEN** user runs `vault ui`
- **THEN** the CLI sends `POST http://127.0.0.1:8765/api/auth/challenge`
- **THEN** vaultd generates a PIN and challenge ID, stores the hash, returns both
- **THEN** the CLI prints the 6-digit PIN to the terminal
- **THEN** the default browser opens `http://127.0.0.1:8765/login?challenge=<id>`

#### Scenario: vaultd not running
- **WHEN** user runs `vault ui` but vaultd is not running on port 8765
- **THEN** the command fails with: "vaultd is not running. Start it with: cargo run -p vaultd"
