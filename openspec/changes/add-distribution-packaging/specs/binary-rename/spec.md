## MODIFIED Requirements

### Requirement: CLI binary named vault
The vault-cli crate SHALL produce a binary named `vault` (not `vault-cli`). The crate package name remains `vault-cli` but the output binary uses `[[bin]] name = "vault"`.

#### Scenario: Binary name after build
- **WHEN** user runs `cargo build -p vault-cli --release`
- **THEN** the output binary is at `target/release/vault`
- **AND** running `./target/release/vault --help` shows the CLI help

#### Scenario: cargo install from git
- **WHEN** user runs `cargo install --git <repo-url> vault-cli`
- **THEN** the installed binary is named `vault`
- **AND** `vault --help` works from PATH

### Requirement: Cargo.toml metadata for crates.io
The vault-cli Cargo.toml SHALL include `description`, `license`, `repository`, `homepage`, and `keywords` fields for discoverability.

#### Scenario: Metadata present
- **WHEN** inspecting vault-cli/Cargo.toml
- **THEN** `description`, `license = "MIT"`, `repository`, and `keywords` fields are present
