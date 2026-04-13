mod commands;
mod support;

use clap::{Parser, Subcommand};
use commands::{
    credential::run_credential_command, profile::run_profile_command, run::run_agent_command,
    stats::run_stats_command,
};

#[derive(Debug, Parser)]
#[command(name = "vault")]
#[command(about = "Local credential broker CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Credential(commands::credential::CredentialCommand),
    Profile(commands::profile::ProfileCommand),
    Run(commands::run::RunCommand),
    Stats(commands::stats::StatsCommand),
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
    }
    Ok(())
}
