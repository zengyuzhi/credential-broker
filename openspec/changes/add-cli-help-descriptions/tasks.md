## Tasks

### Task 1: Add root CLI and top-level subcommand descriptions

**Files:**
- Modify: `crates/vault-cli/src/main.rs`

- [x] Update the `Cli` struct with a descriptive `long_about`
- [x] Add `about` to each variant in the `Command` enum: `Credential`, `Profile`, `Run`, `Stats`
- [x] Run `cargo run -p vault-cli -- --help` and verify all subcommands show descriptions
- [x] Commit: `docs: add root CLI and subcommand help descriptions`

---

### Task 2: Add credential subcommand and argument descriptions

**Files:**
- Modify: `crates/vault-cli/src/commands/credential.rs`

- [x] Add `about` to `CredentialCommand` struct
- [x] Add `about` to each variant in `CredentialSubcommand`: `Add`, `List`, `Disable`, `Enable`, `Remove`
- [x] Add `help` to every `#[arg]` field: `provider`, `label`, `--kind`, `--env` (in `Add`), `id` (in `Disable`/`Enable`/`Remove`), `--yes` (in `Remove`)
- [x] Run `cargo run -p vault-cli -- credential --help` and verify descriptions appear
- [x] Run `cargo run -p vault-cli -- credential add --help` and verify all args have help text
- [x] Commit: `docs: add credential command help descriptions`

---

### Task 3: Add profile subcommand and argument descriptions

**Files:**
- Modify: `crates/vault-cli/src/commands/profile.rs`

- [x] Add `about` to `ProfileCommand` struct
- [x] Add `about` to each variant in `ProfileSubcommand`: `Create`, `List`, `Show`, `Bind`
- [x] Add `help` to every `#[arg]` field: `name` (in `Create`/`Show`), `profile`, `provider`, `credential_id`, `--mode` (in `Bind`)
- [x] Run `cargo run -p vault-cli -- profile --help` and verify descriptions appear
- [x] Run `cargo run -p vault-cli -- profile bind --help` and verify `--mode` lists valid values
- [x] Commit: `docs: add profile command help descriptions`

---

### Task 4: Add run command argument descriptions

**Files:**
- Modify: `crates/vault-cli/src/commands/run.rs`

- [x] Add `about` to `RunCommand` struct (describe: launch agent subprocess with injected credentials)
- [x] Add `help` to `--profile`, `--agent` (include default value), `--project`, and `COMMAND` trailing args
- [x] Run `cargo run -p vault-cli -- run --help` and verify all args have help text
- [x] Commit: `docs: add run command help descriptions`

---

### Task 5: Add stats command argument descriptions

**Files:**
- Modify: `crates/vault-cli/src/commands/stats.rs`

- [x] Add `about` to `StatsCommand` struct (describe: display usage statistics per provider)
- [x] Add `help` to `--provider` (describe: optional filter by provider name)
- [x] Run `cargo run -p vault-cli -- stats --help` and verify args have help text
- [x] Commit: `docs: add stats command help descriptions`

---

### Task 6: Final verification

- [x] Run `cargo clippy --workspace --all-targets -- -D warnings` — must pass clean
- [x] Run `cargo test` — all 49+ tests must pass
- [x] Run `cargo run -p vault-cli -- --help` and verify root output shows `vault` as binary name
- [x] Spot-check: no `--help` output contains `ai.zyr1.vault`, `.local/vault.db`, or any secret values
- [x] Commit any formatting fixes: `chore: fmt after help text additions`
