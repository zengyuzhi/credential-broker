# credential-broker

Local credential broker/vault for coding agents and scripts.

Current skeleton includes:
- Rust workspace with CLI, daemon, and shared crates
- SQLite migration for credentials, profiles, leases, and usage events
- Core domain models and provider adapter trait
- macOS Keychain secret-store scaffold
- Provider registry scaffold for OpenAI, Anthropic, and TwitterAPI
- CLI command skeleton for credential/profile/run/stats flows
- Minimal `vaultd` health and stats routes

Planned V2 features:
- Env injection for agent subprocesses
- Local proxy mode for selected providers
- Usage ledger and rollups
- Policy enforcement and short-lived leases
