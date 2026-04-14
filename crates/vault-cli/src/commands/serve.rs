use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{Context, bail};
use clap::{Args, Subcommand};

use crate::support::config::{current_database_url, resolved_state_dir, state_dir};

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
/// Returns the child PID on success. If the server is already running the
/// existing PID is returned without spawning a second instance.
pub fn spawn_background_server(port: u16) -> anyhow::Result<u32> {
    // Check for an existing running instance.
    if let Some((pid, path)) = read_pid() {
        if is_process_alive(pid) {
            eprintln!("Vault server is already running (pid: {})", pid);
            return Ok(pid);
        }
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

    Ok(pid)
}

// ---------------------------------------------------------------------------
// Health check poller
// ---------------------------------------------------------------------------

/// Poll `/health` until it returns 2xx or the timeout elapses.
/// Returns `true` if the server became healthy within the timeout.
pub async fn wait_for_health(port: u16, timeout_secs: u64) -> bool {
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/health", port);
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(timeout_secs);

    while start.elapsed() < timeout {
        if let Ok(resp) = client.get(&url).send().await
            && resp.status().is_success()
        {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    false
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
                let pid = spawn_background_server(cmd.port)?;
                if !wait_for_health(cmd.port, 5).await {
                    bail!(
                        "Vault server failed to start within 5 seconds (pid: {}). \
                         Check .local/vault.log for details.",
                        pid
                    );
                }
                eprintln!("Vault server started (pid: {})", pid);
                return Ok(());
            }

            // Foreground serve.
            let database_url = current_database_url();
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
