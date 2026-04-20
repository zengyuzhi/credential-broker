use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

use anyhow::{Context, bail};
use clap::{Args, Subcommand};
use vault_db::Store;

use crate::support::config::{current_database_url, resolved_state_dir, state_dir};
use crate::support::keychain_migration::migrate_legacy_credentials_in_store;

const DEFAULT_PORT: u16 = 8765;

#[derive(Debug, Args)]
#[command(about = "Start the vault HTTP server (dashboard and proxy)")]
pub struct ServeCommand {
    #[command(subcommand)]
    pub action: Option<ServeAction>,

    #[arg(long, default_value_t = DEFAULT_PORT, help = "Port to listen on")]
    pub port: u16,

    #[arg(long, help = "Run the server in the background and exit")]
    pub background: bool,
}

#[derive(Debug, Subcommand)]
pub enum ServeAction {
    #[command(about = "Stop a running background server")]
    Stop,
    #[command(about = "Check if the server is running")]
    Status,
}

// ---------------------------------------------------------------------------
// PID file helpers
// ---------------------------------------------------------------------------

fn pid_file_path() -> PathBuf {
    resolved_state_dir().join("vault.pid")
}

fn legacy_pid_file_path() -> Option<PathBuf> {
    std::env::current_dir()
        .ok()
        .map(|current_dir| current_dir.join(".local/vault.pid"))
}

fn read_pid_from_path(path: &PathBuf) -> Option<u32> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn active_pid_path() -> Option<PathBuf> {
    let canonical = pid_file_path();
    if canonical.is_file() {
        return Some(canonical);
    }

    let legacy = legacy_pid_file_path()?;
    legacy.is_file().then_some(legacy)
}

fn read_pid() -> Option<(u32, PathBuf)> {
    let path = active_pid_path()?;
    let pid = read_pid_from_path(&path)?;
    Some((pid, path))
}

fn write_pid(pid: u32) -> anyhow::Result<PathBuf> {
    let path = state_dir().join("vault.pid");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, pid.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(path)
}

fn remove_pid_file(path: &PathBuf) {
    let _ = fs::remove_file(path);
}

fn is_process_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn running_background_server_pid() -> Option<u32> {
    let (pid, path) = read_pid()?;
    if is_process_alive(pid) {
        return Some(pid);
    }

    remove_pid_file(&path);
    None
}

// ---------------------------------------------------------------------------
// Background spawn
// ---------------------------------------------------------------------------

/// Spawn the vault server as a detached background process.
/// Returns the spawned child handle so the caller can confirm that the new
/// process, not an unrelated listener, became healthy.
fn spawn_background_server(port: u16) -> anyhow::Result<Child> {
    if let Some((_, path)) = read_pid() {
        // Stale PID file — clean it up before re-spawning.
        remove_pid_file(&path);
    }

    let exe = std::env::current_exe().context("failed to get current executable path")?;

    let mut cmd = Command::new(&exe);
    cmd.args(["serve", "--port", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null());

    // Place the child in its own process group so it survives parent exit.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    let child = cmd
        .spawn()
        .context("failed to spawn vault server process")?;
    let pid = child.id();

    write_pid(pid)?;

    Ok(child)
}

// ---------------------------------------------------------------------------
// Health check poller
// ---------------------------------------------------------------------------

enum SpawnHealth {
    Ready,
    Exited,
    TimedOut,
}

async fn port_is_healthy(port: u16) -> bool {
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/health", port);
    client
        .get(&url)
        .send()
        .await
        .map(|resp| resp.status().is_success())
        .unwrap_or(false)
}

/// Poll `/health` until it returns 2xx or the timeout elapses.
/// Returns whether the freshly spawned child became healthy, exited early,
/// or simply never became ready within the timeout.
async fn wait_for_spawned_server(
    child: &mut Child,
    port: u16,
    timeout_secs: u64,
) -> anyhow::Result<SpawnHealth> {
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(timeout_secs);

    while start.elapsed() < timeout {
        if child.try_wait()?.is_some() {
            return Ok(SpawnHealth::Exited);
        }
        if port_is_healthy(port).await {
            return Ok(SpawnHealth::Ready);
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    if child.try_wait()?.is_some() {
        return Ok(SpawnHealth::Exited);
    }

    Ok(SpawnHealth::TimedOut)
}

pub async fn ensure_background_server_running(
    port: u16,
    timeout_secs: u64,
) -> anyhow::Result<(u32, bool)> {
    if let Some(pid) = running_background_server_pid() {
        return Ok((pid, false));
    }

    if port_is_healthy(port).await {
        bail!(
            "Port {} is already responding to /health. Stop the existing service or choose another port.",
            port
        );
    }

    let mut child = spawn_background_server(port)?;
    let pid = child.id();

    match wait_for_spawned_server(&mut child, port, timeout_secs).await? {
        SpawnHealth::Ready => Ok((pid, true)),
        SpawnHealth::Exited => {
            remove_pid_file(&pid_file_path());
            bail!(
                "Vault server exited before becoming healthy. Is port {} already in use?",
                port
            );
        }
        SpawnHealth::TimedOut => bail!(
            "Vault server failed to start within {} seconds (pid: {}). Check .local/vault.log for details.",
            timeout_secs,
            pid
        ),
    }
}

// ---------------------------------------------------------------------------
// Command handler
// ---------------------------------------------------------------------------

pub async fn run_serve_command(cmd: ServeCommand) -> anyhow::Result<()> {
    match cmd.action {
        Some(ServeAction::Stop) => match read_pid() {
            Some((pid, path)) if is_process_alive(pid) => {
                let _ = Command::new("kill").arg(pid.to_string()).status();
                remove_pid_file(&path);
                eprintln!("Vault server stopped (pid: {})", pid);
                Ok(())
            }
            Some((_pid, path)) => {
                remove_pid_file(&path);
                eprintln!("Vault server is not running (stale PID file removed)");
                Ok(())
            }
            None => {
                eprintln!("Vault server is not running");
                Ok(())
            }
        },

        Some(ServeAction::Status) => match read_pid() {
            Some((pid, _path)) if is_process_alive(pid) => {
                eprintln!("Vault server is running (pid: {}, port: {})", pid, cmd.port);
                Ok(())
            }
            Some((_pid, path)) => {
                remove_pid_file(&path);
                eprintln!("Vault server is not running (stale PID file cleaned up)");
                Ok(())
            }
            None => {
                eprintln!("Vault server is not running");
                Ok(())
            }
        },

        None => {
            if cmd.background {
                let (pid, started) = ensure_background_server_running(cmd.port, 5).await?;
                if started {
                    eprintln!("Vault server started (pid: {})", pid);
                } else {
                    eprintln!("Vault server is already running (pid: {})", pid);
                }
                return Ok(());
            }

            // Foreground serve.
            let database_url = current_database_url();
            let store = Store::connect(&database_url).await?;
            let _ = migrate_legacy_credentials_in_store(&store).await?;
            eprintln!("Starting vault server on http://127.0.0.1:{}", cmd.port);
            match vaultd::start_server(&database_url, cmd.port).await {
                Ok(()) => Ok(()),
                Err(err) => {
                    let msg = err.to_string();
                    if msg.contains("address already in use")
                        || msg.contains("Address already in use")
                    {
                        bail!(
                            "Port {} is already in use. Is vault serve already running?",
                            cmd.port
                        );
                    }
                    Err(err)
                }
            }
        }
    }
}
