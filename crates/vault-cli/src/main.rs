mod commands;
mod support;

use clap::{Parser, Subcommand};
use commands::{
    credential::run_credential_command, profile::run_profile_command, run::run_agent_command,
    serve::run_serve_command, stats::run_stats_command, ui::run_ui_command,
    upgrade::run_upgrade_command,
};

#[derive(Debug, Parser)]
#[command(name = "vault")]
#[command(version)]
#[command(about = "Local credential broker for coding agents and scripts")]
#[command(
    long_about = "Store API keys securely in macOS Keychain, organize them into named profiles, \
    and launch agent subprocesses with credentials injected as environment variables \
    or forwarded through an authenticated HTTP proxy. Every access is lease-bounded and tracked."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Add, list, enable, disable, or remove stored credentials")]
    Credential(commands::credential::CredentialCommand),
    #[command(about = "Create and manage named profiles that bundle provider credentials")]
    Profile(commands::profile::ProfileCommand),
    #[command(about = "Launch a command with credentials injected from a profile")]
    Run(commands::run::RunCommand),
    #[command(about = "Display usage statistics per provider")]
    Stats(commands::stats::StatsCommand),
    #[command(about = "Open the vault dashboard in your browser")]
    Ui(commands::ui::UiCommand),
    #[command(about = "Start the vault HTTP server (dashboard and proxy)")]
    Serve(commands::serve::ServeCommand),
    #[command(about = "Check for and install a newer vault binary")]
    Upgrade(commands::upgrade::UpgradeCommand),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let cli = Cli::parse();
    match cli.command {
        Command::Credential(cmd) => run_credential_command(cmd).await?,
        Command::Profile(cmd) => run_profile_command(cmd).await?,
        Command::Run(cmd) => run_agent_command(cmd).await?,
        Command::Stats(cmd) => run_stats_command(cmd).await?,
        Command::Ui(cmd) => run_ui_command(cmd).await?,
        Command::Serve(cmd) => run_serve_command(cmd).await?,
        Command::Upgrade(cmd) => run_upgrade_command(cmd).await?,
    }
    Ok(())
}
