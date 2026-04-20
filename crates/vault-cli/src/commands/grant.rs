use anyhow::{Context, bail};
use chrono::{Duration, Utc};
use clap::{Args, Subcommand};
use uuid::Uuid;
use vault_core::models::Grant;
use vault_db::Store;

use crate::support::{config::current_database_url, prompt::print_success};

#[derive(Debug, Args)]
#[command(about = "Manage grants (agent authorizations to use capabilities)")]
pub struct GrantCommand {
    #[command(subcommand)]
    pub command: GrantSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum GrantSubcommand {
    #[command(about = "Create a grant authorizing an agent to use a capability")]
    Add {
        #[arg(long, help = "Agent name (or '*' for any agent)")]
        agent: String,
        #[arg(long, help = "Capability UUID to grant")]
        capability: String,
        #[arg(
            long,
            help = "Optional TTL in minutes for sessions created from this grant"
        )]
        ttl: Option<i64>,
        #[arg(long, help = "Optional max number of requests allowed")]
        max_requests: Option<i64>,
        #[arg(long, help = "Require human confirmation for each use")]
        confirm: bool,
        #[arg(long, help = "Grant expires after this many days")]
        expires_days: Option<i64>,
    },
    #[command(about = "List grants, optionally filtered by agent")]
    List {
        #[arg(long, help = "Filter by agent name")]
        agent: Option<String>,
    },
    #[command(about = "Revoke (delete) a grant by ID")]
    Revoke {
        #[arg(help = "Grant UUID to revoke")]
        id: String,
        #[arg(long, help = "Skip confirmation prompt")]
        yes: bool,
    },
}

pub async fn run_grant_command(cmd: GrantCommand) -> anyhow::Result<()> {
    match cmd.command {
        GrantSubcommand::Add {
            agent,
            capability,
            ttl,
            max_requests,
            confirm,
            expires_days,
        } => {
            add_grant(
                &agent,
                &capability,
                ttl,
                max_requests,
                confirm,
                expires_days,
            )
            .await
        }
        GrantSubcommand::List { agent } => list_grants(agent.as_deref()).await,
        GrantSubcommand::Revoke { id, yes } => revoke_grant(&id, yes).await,
    }
}

async fn add_grant(
    agent_name: &str,
    capability_id_str: &str,
    ttl_minutes: Option<i64>,
    max_requests: Option<i64>,
    require_confirmation: bool,
    expires_days: Option<i64>,
) -> anyhow::Result<()> {
    if let Some(ttl) = ttl_minutes
        && ttl <= 0
    {
        bail!("--ttl must be a positive number of minutes");
    }
    if let Some(days) = expires_days
        && days <= 0
    {
        bail!("--expires-days must be a positive number of days");
    }

    let capability_id = Uuid::parse_str(capability_id_str)
        .with_context(|| format!("invalid capability id: {capability_id_str}"))?;
    let store = Store::connect(&current_database_url()).await?;

    if store.get_capability(capability_id).await?.is_none() {
        bail!("capability not found: {capability_id_str}");
    }

    let now = Utc::now();
    let expires_at = expires_days.map(|d| now + Duration::days(d));

    let grant = Grant {
        id: Uuid::new_v4(),
        agent_name: agent_name.to_string(),
        capability_id,
        ttl_minutes,
        max_requests,
        require_confirmation,
        enabled: true,
        created_at: now,
        expires_at,
    };

    store.insert_grant(&grant).await?;
    print_success(&format!(
        "Added grant id={} agent={} capability={}",
        grant.id, grant.agent_name, grant.capability_id
    ))?;
    Ok(())
}

async fn list_grants(agent_name: Option<&str>) -> anyhow::Result<()> {
    let store = Store::connect(&current_database_url()).await?;

    let grants = if let Some(name) = agent_name {
        store.list_grants_for_agent(name).await?
    } else {
        store.list_grants().await?
    };

    if grants.is_empty() {
        print_success("No grants found.")?;
        return Ok(());
    }

    for g in grants {
        print_success(&format!(
            "id={} agent={} capability={} enabled={} expires={}",
            g.id,
            g.agent_name,
            g.capability_id,
            g.enabled,
            g.expires_at
                .map(|v| v.to_rfc3339())
                .unwrap_or_else(|| "never".to_string())
        ))?;
    }
    Ok(())
}

async fn revoke_grant(id_str: &str, yes: bool) -> anyhow::Result<()> {
    if !yes {
        bail!("pass --yes to confirm revocation of grant '{id_str}'");
    }

    let id = Uuid::parse_str(id_str).with_context(|| format!("invalid grant id: {id_str}"))?;
    let store = Store::connect(&current_database_url()).await?;

    if store.get_grant(id).await?.is_none() {
        bail!("grant not found: {id_str}");
    }

    store.delete_grant(id).await?;
    print_success(&format!("Revoked grant: {id_str}"))?;
    Ok(())
}
