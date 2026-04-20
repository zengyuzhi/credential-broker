## ADDED Requirements

### Requirement: Compatibility-path commands disclose their security posture
The help text in `crates/vault-cli/src/commands/run.rs`, `crates/vault-cli/src/commands/profile.rs`, and any related root-command copy in `crates/vault-cli/src/main.rs` SHALL distinguish env-injection compatibility paths from preferred brokered access. Commands and flags that hand secrets to child processes or configure `inject` / `either` access SHALL disclose that they are weaker than the target brokered model when brokered access is available.

#### Scenario: Run help labels env injection as compatibility access
- **WHEN** a user runs `vault run --help`
- **THEN** the output states that the command launches a child process with injected credentials
- **AND** the output indicates that this is a compatibility path rather than the preferred brokered trust boundary

#### Scenario: Profile bind help distinguishes inject, proxy, and either
- **WHEN** a user runs `vault profile bind --help`
- **THEN** the `--mode` help text describes `inject` as compatibility access
- **AND** the help text describes `proxy` as the preferred brokered path when supported
- **AND** the help text describes `either` as a mixed or transitional mode rather than the long-term default
