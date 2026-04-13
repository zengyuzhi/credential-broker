## ADDED Requirements

### Requirement: Root command displays program description
The root `vault --help` output SHALL display the binary name as `vault` (not `vault-cli`) and include a one-line `about` description and a multi-line `long_about` paragraph explaining the tool's purpose.

#### Scenario: Root help output
- **WHEN** user runs `vault --help`
- **THEN** the output shows `vault` as the program name
- **THEN** the output includes a one-line description mentioning "credential broker"
- **THEN** the output lists all subcommands (`credential`, `profile`, `run`, `stats`) with non-empty descriptions

### Requirement: Every subcommand has an about description
Each subcommand (`credential`, `profile`, `run`, `stats`) and each nested subcommand (`credential add`, `credential list`, `profile bind`, etc.) SHALL have a non-empty `about` description visible in `--help` output.

#### Scenario: Credential subcommand help
- **WHEN** user runs `vault credential --help`
- **THEN** the output shows a description for the `credential` command group
- **THEN** each sub-subcommand (`add`, `list`, `enable`, `disable`, `remove`) has a non-empty description

#### Scenario: Profile subcommand help
- **WHEN** user runs `vault profile --help`
- **THEN** each sub-subcommand (`create`, `list`, `show`, `bind`) has a non-empty description

#### Scenario: Run subcommand help
- **WHEN** user runs `vault run --help`
- **THEN** the output describes the purpose of `run` (launching agent subprocesses with injected credentials)

#### Scenario: Stats subcommand help
- **WHEN** user runs `vault stats --help`
- **THEN** the output describes the purpose of `stats` (displaying usage statistics)

### Requirement: Every argument and flag has help text
Every `#[arg]` field across all commands SHALL have a non-empty `help` attribute describing its purpose, expected values, and default (if any).

#### Scenario: Credential add arguments
- **WHEN** user runs `vault credential add --help`
- **THEN** `provider` shows help text explaining it accepts a provider name (e.g., openai, anthropic)
- **THEN** `label` shows help text explaining it is a human-readable name for the credential
- **THEN** `--kind` shows help text including the default value `api_key`
- **THEN** `--env` shows help text including the default value `work`

#### Scenario: Run command arguments
- **WHEN** user runs `vault run --help`
- **THEN** `--profile` shows help text explaining it selects the named profile
- **THEN** `--agent` shows help text explaining it identifies the calling agent, with default `unknown-agent`
- **THEN** `--project` shows help text explaining the optional project identifier
- **THEN** `COMMAND` shows help text explaining it is the command to execute with injected credentials

#### Scenario: Profile bind arguments
- **WHEN** user runs `vault profile bind --help`
- **THEN** `profile` shows help text explaining it is the profile name
- **THEN** `provider` shows help text explaining it is the provider to bind
- **THEN** `credential_id` shows help text explaining it is the credential UUID
- **THEN** `--mode` shows help text listing valid values: `inject`, `proxy`, `either`

### Requirement: Help text does not expose secrets or internal paths
No `--help` output SHALL contain actual secret values, database paths, or Keychain service names.

#### Scenario: No sensitive data in help
- **WHEN** user runs `vault --help` or any subcommand `--help`
- **THEN** the output does not contain `ai.zyr1.vault`, `.local/vault.db`, or any API key patterns
