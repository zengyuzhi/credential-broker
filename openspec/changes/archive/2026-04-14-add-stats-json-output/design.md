## Context

`vault stats` currently prints `key=value` text via `println!`. The `ProviderStats` struct in vault-db lacks `Serialize`, so it can't be directly serialized to JSON.

## Goals / Non-Goals

**Goals:**
- Add `--json` flag for machine-readable output
- Keep default text output unchanged

**Non-Goals:**
- Pretty-printing or formatting options beyond JSON
- Streaming output or pagination

## Decisions

### 1. Add Serialize to ProviderStats rather than creating a separate DTO

Rationale: `ProviderStats` already has the exact fields we want in the JSON output. Adding `#[derive(Serialize)]` is one line and avoids a mapping layer. The `serde` dependency is already in vault-db's Cargo.toml.

### 2. Filter before serialization

Rationale: Apply the `--provider` filter to the stats vec before serializing, so `--json --provider openai` outputs a filtered array. This matches the text behavior where non-matching rows are skipped.
