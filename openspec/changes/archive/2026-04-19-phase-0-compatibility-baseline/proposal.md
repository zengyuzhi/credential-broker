## Why

The project now has a clearer first-principles direction: evolve from a credential launcher toward a broker-first local API control plane. Before Phase 1 introduces new domain concepts such as connectors, grants, and bundles, Phase 0 needs to stabilize the current product as an explicit compatibility baseline and label its weaker paths honestly.

## What Changes

- Publish Phase 0 as a non-breaking compatibility-baseline change rather than an architectural rewrite.
- Add a new capability spec that defines the Phase 0 contract: preserve the current credential/profile/run/serve/ui/upgrade surfaces while documenting the target broker-first direction.
- Extend CLI help requirements so commands that inject secrets into child processes disclose that they are compatibility paths, while brokered access is the preferred direction when supported.
- Align README, architecture docs, and future user-facing help text around the same terminology so users are not told that env injection and brokered access are equally strong security models.
- Add regression-oriented verification so Phase 0 guidance changes do not silently break existing workflows.

## Capabilities

### New Capabilities
- `compatibility-baseline`: Defines the Phase 0 contract for preserving the current product surfaces while clearly distinguishing compatibility access from the target broker-first architecture.

### Modified Capabilities
- `cli-help-text`: Add explicit compatibility and security guidance for commands and flags that inject secrets into child processes or configure inject/either access modes.

## Impact

- **Affected crates**:
  - `vault-cli`: help text for root and subcommand surfaces, especially `crates/vault-cli/src/main.rs`, `crates/vault-cli/src/commands/run.rs`, and `crates/vault-cli/src/commands/profile.rs`
  - documentation surfaces rooted at `README.md`, `docs/ARCHITECTURE.md`, and `docs/plans/`
- **Affected systems**:
  - user-facing CLI guidance
  - repository architecture and rollout documentation
  - regression verification for current credential/profile/run/serve/ui workflows
- **No intended impact**:
  - no new database objects
  - no migration of stored credentials
  - no immediate replacement of the current profile/run model

## Security Implications

This change is primarily about security honesty. It does not make env injection stronger; instead, it reduces the chance that users or future contributors mistake a compatibility path for the preferred brokered trust boundary.

## Out of Scope

- Introducing connectors, grants, sessions, or bundles as runtime primitives
- Replacing the current `credential` / `profile` / `run` data model
- Building the preset catalog, curl import, or OpenAPI import flows
- Removing the current CLI surfaces or forcing users onto a new model during Phase 0
