use std::num::NonZeroU32;

use anyhow::{Context, bail};
use clap::{Args, Subcommand};
use uuid::Uuid;
use vault_db::Store;
use vault_policy::{service::PolicyService, session::issue_session};
use zeroize::Zeroizing;

use crate::support::{config::current_database_url, prompt::print_success};

const MAX_SESSION_TTL_MINUTES: u32 = 10_080; // 1 week

#[derive(Debug, Args)]
#[command(about = "Manage broker sessions (short-lived scoped tokens)")]
pub struct SessionCommand {
    #[command(subcommand)]
    pub command: SessionSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum SessionSubcommand {
    #[command(about = "Issue a new session token scoped to a bundle")]
    Issue {
        #[arg(long, help = "Bundle name to scope the session to")]
        bundle: String,
        #[arg(long, help = "Agent name this session is issued for")]
        agent: String,
        #[arg(long, help = "Project context tag")]
        project: Option<String>,
        #[arg(long, default_value = "60", help = "Session TTL in minutes")]
        ttl: u32,
    },
    #[command(about = "List active sessions")]
    List,
    #[command(about = "Revoke (delete) a session by ID")]
    Revoke {
        #[arg(help = "Session UUID to revoke")]
        id: String,
    },
}

pub async fn run_session_command(cmd: SessionCommand) -> anyhow::Result<()> {
    match cmd.command {
        SessionSubcommand::Issue {
            bundle,
            agent,
            project,
            ttl,
        } => issue(&bundle, &agent, project, ttl).await,
        SessionSubcommand::List => list_sessions().await,
        SessionSubcommand::Revoke { id } => revoke_session(&id).await,
    }
}

async fn issue(
    bundle_name: &str,
    agent_name: &str,
    project: Option<String>,
    ttl: u32,
) -> anyhow::Result<()> {
    if ttl > MAX_SESSION_TTL_MINUTES {
        bail!(
            "TTL {} exceeds maximum of {} minutes (1 week)",
            ttl,
            MAX_SESSION_TTL_MINUTES
        );
    }
    let ttl_minutes = NonZeroU32::new(ttl).with_context(|| "TTL must be greater than zero")?;
    let store = Store::connect(&current_database_url()).await?;

    let bundle = store
        .get_bundle_by_name(bundle_name)
        .await?
        .with_context(|| format!("bundle not found: {bundle_name}"))?;

    let grants = store.list_grants_for_bundle(bundle.id).await?;
    if grants.is_empty() {
        bail!("bundle '{bundle_name}' has no grants — cannot issue session");
    }

    let policy = PolicyService::default();
    let active_grants: Vec<_> = grants
        .iter()
        .filter(|g| policy.check_grant_active(g).is_ok())
        .collect();
    if active_grants.is_empty() {
        bail!("bundle '{bundle_name}' has no active grants — all are disabled or expired");
    }

    let (session, raw_token) = issue_session(Some(bundle.id), agent_name, project, ttl_minutes);

    let grant_ids: Vec<Uuid> = active_grants.iter().map(|g| g.id).collect();
    store
        .insert_session_with_grants(&session, &grant_ids)
        .await?;

    print_success(&format!(
        "Issued session id={} bundle={} agent={} expires={}",
        session.id,
        bundle_name,
        session.agent_name,
        session.expires_at.to_rfc3339(),
    ))?;
    let token_line = Zeroizing::new(format!("token={}", raw_token.as_str()));
    print_success(token_line.as_str())?;
    Ok(())
}

async fn list_sessions() -> anyhow::Result<()> {
    let store = Store::connect(&current_database_url()).await?;
    let sessions = store.list_active_sessions().await?;

    if sessions.is_empty() {
        print_success("No active sessions.")?;
        return Ok(());
    }

    for s in sessions {
        print_success(&format!(
            "id={} agent={} bundle={} requests={} expires={}",
            s.id,
            s.agent_name,
            s.bundle_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "(none)".to_string()),
            s.request_count,
            s.expires_at.to_rfc3339()
        ))?;
    }
    Ok(())
}

async fn revoke_session(id_str: &str) -> anyhow::Result<()> {
    let id = Uuid::parse_str(id_str).with_context(|| format!("invalid session id: {id_str}"))?;
    let store = Store::connect(&current_database_url()).await?;

    if store.get_session(id).await?.is_none() {
        bail!("session not found: {id_str}");
    }

    store.delete_session(id).await?;
    print_success(&format!("Revoked session: {id_str}"))?;
    Ok(())
}
