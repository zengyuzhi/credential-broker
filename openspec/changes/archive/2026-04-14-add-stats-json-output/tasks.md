## Tasks

### 1. Add Serialize to ProviderStats

- [x] Add `#[derive(Serialize)]` to `ProviderStats` in `crates/vault-db/src/usage_events.rs`
- [x] Add `use serde::Serialize;` import
- [x] Add `serde.workspace = true` to vault-db Cargo.toml

### 2. Add --json flag and output branch

- [x] Add `#[arg(long, help = "Output stats as JSON array")]` field `json: bool` to `StatsCommand`
- [x] Filter stats vec by `--provider` before output
- [x] If `json` flag is set, serialize filtered stats via `serde_json::to_string_pretty` and print
- [x] If `json` flag is not set, print existing key=value text format
- [x] If no data and `--json`, print `[]`

### 3. Verify

- [x] Run `cargo build -p vault-cli`
- [x] Run `cargo clippy --workspace --all-targets -- -D warnings`
- [x] Run `cargo test --workspace`
