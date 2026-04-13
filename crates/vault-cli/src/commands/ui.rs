use anyhow::{bail, Context};
use clap::Args;
use serde::Deserialize;

use crate::support::prompt::print_success;

const DEFAULT_PORT: u16 = 8765;
const VAULTD_BASE: &str = "http://127.0.0.1:8765";

#[derive(Debug, Args)]
#[command(about = "Open the vault dashboard in your browser")]
pub struct UiCommand {}

#[derive(Debug, Deserialize)]
struct ChallengeResponse {
    challenge_id: String,
    pin: String,
}

pub async fn run_ui_command(_cmd: UiCommand) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let health_url = format!("{VAULTD_BASE}/health");

    // Check if daemon is running; auto-start if not.
    let is_running = client
        .get(&health_url)
        .send()
        .await
        .map(|resp| resp.status().is_success())
        .unwrap_or(false);

    if !is_running {
        let pid = crate::commands::serve::spawn_background_server(DEFAULT_PORT)?;
        eprintln!("Started vault server in background (pid: {pid})");
        if !crate::commands::serve::wait_for_health(DEFAULT_PORT, 5).await {
            bail!(
                "Could not start vault server. Check port {DEFAULT_PORT} or run `vault serve` manually."
            );
        }
    }

    // 2. Request a challenge (PIN)
    let challenge_url = format!("{VAULTD_BASE}/api/auth/challenge");
    let resp = client
        .post(&challenge_url)
        .send()
        .await
        .context("failed to request auth challenge")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        bail!("challenge request failed ({}): {}", status, body);
    }

    let challenge: ChallengeResponse = resp
        .json()
        .await
        .context("failed to parse challenge response")?;

    // 3. Print PIN to terminal
    print_success(&format!(
        "Dashboard PIN: {}\n  This PIN expires in 5 minutes. Enter it in the browser to log in.",
        challenge.pin
    ))?;

    // 4. Open browser
    let login_url = format!("{VAULTD_BASE}/login?challenge={}", challenge.challenge_id);

    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("open")
            .arg(&login_url)
            .status()
            .context("failed to open browser")?;
        if !status.success() {
            eprintln!("Could not open browser automatically. Visit: {login_url}");
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("Open this URL in your browser: {login_url}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vaultd_base_url_is_localhost() {
        assert!(VAULTD_BASE.starts_with("http://127.0.0.1"));
    }
}
