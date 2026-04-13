## Why

The `vault stats` command outputs raw `key=value` text that isn't machine-parseable. Scripts and integrations (CI pipelines, cost dashboards, monitoring) can't easily consume it. Adding a `--json` flag enables programmatic access to usage data without scraping text output.

## What Changes

- Add `--json` boolean flag to `vault stats`
- When `--json` is set, output a JSON array of provider stats instead of text lines
- Add `Serialize` derive to `ProviderStats` in `vault-db`

## Capabilities

### Modified Capabilities
- `stats-cli-output`: Machine-readable JSON output mode for vault stats

## Impact

- `crates/vault-cli/src/commands/stats.rs`: Add flag and JSON branch
- `crates/vault-db/src/usage_events.rs`: Add `Serialize` derive to `ProviderStats`
