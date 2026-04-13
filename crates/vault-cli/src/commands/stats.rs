use clap::Args;

#[derive(Debug, Args)]
pub struct StatsCommand {
    #[arg(long)]
    pub provider: Option<String>,
    #[arg(long)]
    pub profile: Option<String>,
}

pub async fn run_stats_command(cmd: StatsCommand) -> anyhow::Result<()> {
    println!(
        "TODO: stats provider={:?} profile={:?}",
        cmd.provider, cmd.profile
    );
    Ok(())
}
