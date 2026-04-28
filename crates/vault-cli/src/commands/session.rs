use std::num::NonZeroU32;

use anyhow::{Context, bail};
use clap::{Args, Subcommand};
use uuid::Uuid;
use vault_db::Store;
use vault_policy::session::{clamp_session_ttl, issue_session, select_attachable_grants};
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

    // Phase 1.1: select only grants scoped to this agent (wildcard `"*"`
    // matches), active (enabled + not expired), and not requiring
    // confirmation. Prevents cross-agent privilege leaks and keeps
    // confirmation-required grants off unattended sessions until the
    // interactive approval flow exists.
    let selection = select_attachable_grants(&grants, agent_name);

    if !selection.skipped_confirmation.is_empty() {
        let skipped: Vec<String> = selection
            .skipped_confirmation
            .iter()
            .map(|g| g.id.to_string())
            .collect();
        print_success(&format!(
            "Note: skipped {} confirmation-required grant(s); interactive approval is not yet supported: {}",
            skipped.len(),
            skipped.join(", ")
        ))?;
    }

    if selection.attachable.is_empty() {
        bail!(
            "bundle '{bundle_name}' has no attachable grants for agent '{agent_name}' — \
             all matching grants are disabled, expired, scoped to another agent, or require confirmation"
        );
    }

    // Phase 1.1: cap the effective session TTL at the tightest
    // `grant.ttl_minutes` across attached grants.
    let (effective_ttl, clamped_to) = clamp_session_ttl(ttl_minutes, &selection.attachable);
    if let Some(cap) = clamped_to {
        print_success(&format!(
            "Note: capping session TTL to {cap} minutes (tightest grant.ttl_minutes cap)"
        ))?;
    }

    let (session, raw_token) = issue_session(Some(bundle.id), agent_name, project, effective_ttl);

    let grant_ids: Vec<Uuid> = selection.attachable.iter().map(|g| g.id).collect();
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
    print_success(
        "Use: curl -H 'Authorization: Bearer <token>' -H 'Content-Type: application/json' \
         -d @body.json http://127.0.0.1:8765/v1/proxy/<provider>/<path>",
    )?;
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
