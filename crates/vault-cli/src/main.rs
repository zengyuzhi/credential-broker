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
    long_about = "Store API keys securely in macOS Keychain, keep today's profile-based \
    compatibility workflows available, and prefer brokered HTTP access when a tool can use the \
    local vault directly. Env injection remains supported for compatibility, and every access is \
    lease-bounded and tracked."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Add, list, enable, disable, or remove stored credentials")]
    Credential(commands::credential::CredentialCommand),
    #[command(about = "Create and manage named profiles for compatibility and brokered workflows")]
    Profile(commands::profile::ProfileCommand),
    #[command(
        about = "Launch a command with profile credentials via env injection (compatibility path)"
    )]
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

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Cli;

    fn render_help(cmd: &mut clap::Command) -> String {
        let mut bytes = Vec::new();
        cmd.write_long_help(&mut bytes).expect("write help");
        String::from_utf8(bytes).expect("help should be valid utf8")
    }

    #[test]
    fn root_help_mentions_brokered_access_and_compatibility() {
        let mut cmd = Cli::command();
        let help = render_help(&mut cmd);

        assert!(help.contains("brokered HTTP access"));
        assert!(help.contains("Env injection remains supported for compatibility"));
    }

    #[test]
    fn run_help_labels_env_injection_as_compatibility_path() {
        let mut cmd = Cli::command();
        let run = cmd.find_subcommand_mut("run").expect("run subcommand");
        let help = render_help(run);

        assert!(help.contains("env injection"));
        assert!(help.contains("supported for compatibility"));
        assert!(help.contains("preferred path"));
    }

    #[test]
    fn profile_bind_help_distinguishes_modes() {
        let mut cmd = Cli::command();
        let profile = cmd
            .find_subcommand_mut("profile")
            .expect("profile subcommand");
        let bind = profile
            .find_subcommand_mut("bind")
            .expect("bind subcommand");
        let help = render_help(bind);

        assert!(help.contains("inject (compatibility env vars)"));
        assert!(help.contains("proxy (preferred brokered HTTP forwarding when supported)"));
        assert!(help.contains("either (mixed transitional mode)"));
    }
}
