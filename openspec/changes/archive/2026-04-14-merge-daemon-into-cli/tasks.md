## Tasks

### Task 1: Convert vaultd from binary to library

**Files:**
- Create: `crates/vaultd/src/lib.rs`
- Modify: `crates/vaultd/src/main.rs`
- Modify: `crates/vaultd/Cargo.toml`

- [x] Extract server startup logic into `pub async fn start_server(database_url: &str, port: u16) -> anyhow::Result<()>` in `lib.rs`
- [x] Make `app`, `auth`, `routes`, `static_assets` modules public in `lib.rs`
- [x] Reduce `main.rs` to a thin wrapper calling `vaultd::start_server()`
- [x] Verify `cargo run -p vaultd` still works
- [x] Commit: `refactor: extract vaultd server startup into library function`

---

### Task 2: Add vault serve subcommand (foreground mode)

**Files:**
- Create: `crates/vault-cli/src/commands/serve.rs`
- Modify: `crates/vault-cli/src/commands/mod.rs`
- Modify: `crates/vault-cli/src/main.rs`
- Modify: `crates/vault-cli/Cargo.toml`

- [x] Add `vaultd` as a dependency of vault-cli
- [x] Create `ServeCommand` with `--port` option (default 8765)
- [x] Implement foreground serve: call `vaultd::start_server()` and block
- [x] Add `Serve` variant to CLI `Command` enum and wire in main.rs
- [x] Handle port-in-use error with clear message
- [x] Commit: `feat: add vault serve subcommand`

---

### Task 3: Add background mode and PID file management

**Files:**
- Modify: `crates/vault-cli/src/commands/serve.rs`

- [x] Add `--background` flag to `ServeCommand`
- [x] Implement `spawn_background_server()`: re-exec `vault serve` via `std::process::Command`
- [x] Detach child process (Unix: process_group(0))
- [x] Write child PID to `.local/vault.pid`
- [x] Wait for `/health` (poll every 200ms, timeout 5s) before exiting
- [x] Print "Vault server started (pid: XXXX)"
- [x] Handle already-running case: check PID file + process alive → print message and exit ok
- [x] Commit: `feat: add vault serve background mode, stop, and status`

---

### Task 4: Add vault serve stop and vault serve status

**Files:**
- Modify: `crates/vault-cli/src/commands/serve.rs`

- [x] Add `ServeAction` enum: Stop, Status
- [x] Implement `stop`: read `.local/vault.pid`, send SIGTERM via kill, remove PID file
- [x] Implement `status`: read PID file, check process alive, print status
- [x] Handle stale PID file (process dead but file exists) → clean up file
- [x] Commit: `feat: add vault serve background mode, stop, and status`

---

### Task 5: Make vault ui auto-start the daemon

**Files:**
- Modify: `crates/vault-cli/src/commands/ui.rs`

- [x] When `/health` probe fails, call `spawn_background_server()` from serve module
- [x] Wait for health check (up to 5s)
- [x] If start succeeds, print "Started vault server in background (pid: XXXX)" then continue
- [x] If start fails after timeout, bail with clear error message
- [x] Remove the current "Cannot reach vaultd" bail — replace with auto-start logic
- [x] Commit: `feat: vault ui auto-starts daemon when not running`

---

### Task 6: Deprecate standalone vaultd binary

**Files:**
- Modify: `crates/vaultd/src/main.rs`
- Modify: `CLAUDE.md`

- [x] Keep `main.rs` for backwards compat with deprecation notice
- [x] Add deprecation eprintln pointing to `vault serve`
- [x] Update CLAUDE.md with new commands
- [x] Commit: `chore: deprecate standalone vaultd binary in favor of vault serve`

---

### Task 7: Verify and test

- [x] Run `cargo build --workspace` — clean
- [x] Run `cargo clippy --workspace --all-targets -- -D warnings` — clean
- [x] Run `cargo test --workspace` — all pass
- [ ] Manual test: `vault serve --background` → `vault ui` → browse dashboard → `vault serve stop`
- [ ] Manual test: `vault ui` from cold start (no daemon running) → auto-starts → opens browser
