use anyhow::{Context, bail};
use chrono::Utc;
use clap::{Args, Subcommand};
use uuid::Uuid;
use vault_core::models::Connector;
use vault_db::Store;

use crate::support::{config::current_database_url, prompt::print_success};

#[derive(Debug, Args)]
#[command(about = "Manage connectors (configured upstream API connections)")]
pub struct ConnectorCommand {
    #[command(subcommand)]
    pub command: ConnectorSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ConnectorSubcommand {
    #[command(about = "Register a new connector backed by an existing credential")]
    Add {
        #[arg(help = "Unique connector name (e.g. my-openai)")]
        name: String,
        #[arg(long, help = "Provider name (e.g. openai, anthropic)")]
        provider: String,
        #[arg(long, help = "Credential UUID that backs this connector")]
        credential: String,
        #[arg(long, help = "Optional base URL override for the upstream API")]
        base_url: Option<String>,
    },
    #[command(about = "List all connectors")]
    List,
    #[command(about = "Show details for a connector by name")]
    Show {
        #[arg(help = "Connector name")]
        name: String,
    },
    #[command(about = "Enable a connector")]
    Enable {
        #[arg(help = "Connector name")]
        name: String,
    },
    #[command(about = "Disable a connector")]
    Disable {
        #[arg(help = "Connector name")]
        name: String,
    },
    #[command(about = "Remove a connector and its capabilities")]
    Remove {
        #[arg(help = "Connector name")]
        name: String,
        #[arg(long, help = "Skip confirmation prompt")]
        yes: bool,
    },
}

pub async fn run_connector_command(cmd: ConnectorCommand) -> anyhow::Result<()> {
    match cmd.command {
        ConnectorSubcommand::Add {
            name,
            provider,
            credential,
            base_url,
        } => add_connector(&name, &provider, &credential, base_url.as_deref()).await,
        ConnectorSubcommand::List => list_connectors().await,
        ConnectorSubcommand::Show { name } => show_connector(&name).await,
        ConnectorSubcommand::Enable { name } => set_connector_enabled(&name, true).await,
        ConnectorSubcommand::Disable { name } => set_connector_enabled(&name, false).await,
        ConnectorSubcommand::Remove { name, yes } => remove_connector(&name, yes).await,
    }
}

async fn add_connector(
    name: &str,
    provider: &str,
    credential_id_str: &str,
    base_url: Option<&str>,
) -> anyhow::Result<()> {
    let credential_id = Uuid::parse_str(credential_id_str)
        .with_context(|| format!("invalid credential id: {credential_id_str}"))?;
    let store = Store::connect(&current_database_url()).await?;

    let cred = store.get_credential(credential_id).await?;
    if cred.is_none() {
        bail!("credential not found: {credential_id_str}");
    }

    if store.get_connector_by_name(name).await?.is_some() {
        bail!("connector with name '{name}' already exists");
    }

    let now = Utc::now();
    let connector = Connector {
        id: Uuid::new_v4(),
        name: name.to_string(),
        provider: provider.to_string(),
        credential_id,
        base_url: base_url.map(String::from),
        enabled: true,
        created_at: now,
        updated_at: now,
    };

    store.insert_connector(&connector).await?;
    print_success(&format!(
        "Added connector id={} name={} provider={} credential={}",
        connector.id, connector.name, connector.provider, connector.credential_id
    ))?;
    Ok(())
}

async fn list_connectors() -> anyhow::Result<()> {
    let store = Store::connect(&current_database_url()).await?;
    let connectors = store.list_connectors().await?;

    if connectors.is_empty() {
        print_success("No connectors registered.")?;
        return Ok(());
    }

    for c in connectors {
        print_success(&format!(
            "id={} name={} provider={} credential={} enabled={}",
            c.id, c.name, c.provider, c.credential_id, c.enabled
        ))?;
    }
    Ok(())
}

async fn show_connector(name: &str) -> anyhow::Result<()> {
    let store = Store::connect(&current_database_url()).await?;
    let connector = store
        .get_connector_by_name(name)
        .await?
        .with_context(|| format!("connector not found: {name}"))?;

    print_success(&format!(
        "id={}\nname={}\nprovider={}\ncredential_id={}\nbase_url={}\nenabled={}\ncreated_at={}\nupdated_at={}",
        connector.id,
        connector.name,
        connector.provider,
        connector.credential_id,
        connector.base_url.as_deref().unwrap_or("(default)"),
        connector.enabled,
        connector.created_at.to_rfc3339(),
        connector.updated_at.to_rfc3339(),
    ))?;
    Ok(())
}

async fn set_connector_enabled(name: &str, enabled: bool) -> anyhow::Result<()> {
    let store = Store::connect(&current_database_url()).await?;
    let connector = store
        .get_connector_by_name(name)
        .await?
        .with_context(|| format!("connector not found: {name}"))?;

    store.set_connector_enabled(connector.id, enabled).await?;
    let label = if enabled { "Enabled" } else { "Disabled" };
    print_success(&format!("{label} connector: {name}"))?;
    Ok(())
}

async fn remove_connector(name: &str, yes: bool) -> anyhow::Result<()> {
    if !yes {
        bail!("pass --yes to confirm deletion of connector '{name}' and all its capabilities");
    }

    let store = Store::connect(&current_database_url()).await?;
    let connector = store
        .get_connector_by_name(name)
        .await?
        .with_context(|| format!("connector not found: {name}"))?;

    store.delete_connector(connector.id).await?;
    print_success(&format!("Removed connector: {name}"))?;
    Ok(())
}
