use anyhow::bail;
use clap::{Args, Subcommand};

use crate::support::config::current_database_url;

const DEFAULT_PORT: u16 = 8765;

#[derive(Debug, Args)]
#[command(about = "Start the vault HTTP server (dashboard and proxy)")]
pub struct ServeCommand {
    #[command(subcommand)]
    pub action: Option<ServeAction>,

    #[arg(long, default_value_t = DEFAULT_PORT, help = "Port to listen on")]
    pub port: u16,
}

#[derive(Debug, Subcommand)]
pub enum ServeAction {
    #[command(about = "Stop a running background server")]
    Stop,
    #[command(about = "Check if the server is running")]
    Status,
}

pub async fn run_serve_command(cmd: ServeCommand) -> anyhow::Result<()> {
    match cmd.action {
        Some(ServeAction::Stop) => {
            // Task 4 will implement this
            bail!("vault serve stop is not yet implemented");
        }
        Some(ServeAction::Status) => {
            // Task 4 will implement this
            bail!("vault serve status is not yet implemented");
        }
        None => {
            // Foreground serve
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
