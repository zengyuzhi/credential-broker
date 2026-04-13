use anyhow::{bail, Context};
use clap::Args;
use serde::Deserialize;

use crate::support::prompt::print_success;

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
    // 1. Check daemon is running via /health
    let client = reqwest::Client::new();
    let health_url = format!("{VAULTD_BASE}/health");

    let health_result = client.get(&health_url).send().await;
    match health_result {
        Ok(resp) if resp.status().is_success() => {}
        Ok(resp) => bail!("vaultd returned unexpected status: {}", resp.status()),
        Err(err) => {
            bail!(
                "Cannot reach vaultd at {VAULTD_BASE}.\n\
                 Make sure the daemon is running: cargo run -p vaultd\n\
                 Underlying error: {err}"
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
