use anyhow::{Context, bail};
use chrono::Utc;
use clap::{Args, Subcommand};
use uuid::Uuid;
use vault_core::models::Capability;
use vault_db::Store;

use crate::support::{config::current_database_url, prompt::print_success};

#[derive(Debug, Args)]
#[command(about = "Manage capabilities (named actions a connector exposes)")]
pub struct CapabilityCommand {
    #[command(subcommand)]
    pub command: CapabilitySubcommand,
}

#[derive(Debug, Subcommand)]
pub enum CapabilitySubcommand {
    #[command(about = "Add a named capability to a connector")]
    Add {
        #[arg(help = "Capability name (e.g. openai.chat, anthropic.messages)")]
        name: String,
        #[arg(long, help = "Connector name that exposes this capability")]
        connector: String,
        #[arg(long, help = "Human-readable description")]
        description: Option<String>,
    },
    #[command(about = "List capabilities, optionally filtered by connector")]
    List {
        #[arg(long, help = "Filter by connector name")]
        connector: Option<String>,
    },
    #[command(about = "Remove a capability by ID")]
    Remove {
        #[arg(help = "Capability UUID")]
        id: String,
        #[arg(long, help = "Skip confirmation prompt")]
        yes: bool,
    },
}

pub async fn run_capability_command(cmd: CapabilityCommand) -> anyhow::Result<()> {
    match cmd.command {
        CapabilitySubcommand::Add {
            name,
            connector,
            description,
        } => add_capability(&name, &connector, description.as_deref()).await,
        CapabilitySubcommand::List { connector } => list_capabilities(connector.as_deref()).await,
        CapabilitySubcommand::Remove { id, yes } => remove_capability(&id, yes).await,
    }
}

async fn add_capability(
    name: &str,
    connector_name: &str,
    description: Option<&str>,
) -> anyhow::Result<()> {
    let store = Store::connect(&current_database_url()).await?;
    let connector = store
        .get_connector_by_name(connector_name)
        .await?
        .with_context(|| format!("connector not found: {connector_name}"))?;

    if store
        .get_capability_by_name(connector.id, name)
        .await?
        .is_some()
    {
        bail!("capability '{name}' already exists on connector '{connector_name}'");
    }

    let capability = Capability {
        id: Uuid::new_v4(),
        connector_id: connector.id,
        name: name.to_string(),
        description: description.map(String::from),
        enabled: true,
        created_at: Utc::now(),
    };

    store.insert_capability(&capability).await?;
    print_success(&format!(
        "Added capability id={} name={} connector={}",
        capability.id, capability.name, connector_name
    ))?;
    Ok(())
}

async fn list_capabilities(connector_name: Option<&str>) -> anyhow::Result<()> {
    let store = Store::connect(&current_database_url()).await?;

    let capabilities = if let Some(cname) = connector_name {
        let connector = store
            .get_connector_by_name(cname)
            .await?
            .with_context(|| format!("connector not found: {cname}"))?;
        store.list_capabilities_for_connector(connector.id).await?
    } else {
        store.list_capabilities().await?
    };

    if capabilities.is_empty() {
        print_success("No capabilities found.")?;
        return Ok(());
    }

    for cap in capabilities {
        print_success(&format!(
            "id={} name={} connector={} enabled={}",
            cap.id, cap.name, cap.connector_id, cap.enabled
        ))?;
    }
    Ok(())
}

async fn remove_capability(id_str: &str, yes: bool) -> anyhow::Result<()> {
    if !yes {
        bail!("pass --yes to confirm deletion of capability '{id_str}' and its grants");
    }

    let id = Uuid::parse_str(id_str).with_context(|| format!("invalid capability id: {id_str}"))?;
    let store = Store::connect(&current_database_url()).await?;

    let cap = store.get_capability(id).await?;
    if cap.is_none() {
        bail!("capability not found: {id_str}");
    }

    store.delete_capability(id).await?;
    print_success(&format!("Removed capability: {id_str}"))?;
    Ok(())
}
