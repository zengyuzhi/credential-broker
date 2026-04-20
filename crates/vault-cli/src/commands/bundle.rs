use anyhow::{Context, bail};
use chrono::Utc;
use clap::{Args, Subcommand};
use uuid::Uuid;
use vault_core::models::{Bundle, Capability, Connector, Grant};
use vault_db::Store;
use vault_policy::service::PolicyService;

use crate::support::{config::current_database_url, prompt::print_success};

#[derive(Debug, Args)]
#[command(about = "Manage bundles (named groupings of grants, evolving from profiles)")]
pub struct BundleCommand {
    #[command(subcommand)]
    pub command: BundleSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum BundleSubcommand {
    #[command(about = "Create a new empty bundle")]
    Create {
        #[arg(help = "Unique bundle name")]
        name: String,
        #[arg(long, help = "Description")]
        description: Option<String>,
    },
    #[command(about = "List all bundles")]
    List,
    #[command(about = "Show bundle details and its grants")]
    Show {
        #[arg(help = "Bundle name")]
        name: String,
    },
    #[command(about = "Add a grant to a bundle")]
    AddGrant {
        #[arg(help = "Bundle name")]
        bundle: String,
        #[arg(help = "Grant UUID to add")]
        grant_id: String,
    },
    #[command(about = "Convert an existing profile into a bundle (compatibility bridge)")]
    FromProfile {
        #[arg(help = "Profile name to convert")]
        profile: String,
        #[arg(long, help = "Name for the new bundle (defaults to profile name)")]
        name: Option<String>,
        #[arg(long, help = "Confirm creation of wildcard grants for all bindings")]
        yes: bool,
    },
    #[command(about = "Remove a bundle")]
    Remove {
        #[arg(help = "Bundle name")]
        name: String,
        #[arg(long, help = "Skip confirmation prompt")]
        yes: bool,
    },
}

pub async fn run_bundle_command(cmd: BundleCommand) -> anyhow::Result<()> {
    match cmd.command {
        BundleSubcommand::Create { name, description } => {
            create_bundle(&name, description.as_deref()).await
        }
        BundleSubcommand::List => list_bundles().await,
        BundleSubcommand::Show { name } => show_bundle(&name).await,
        BundleSubcommand::AddGrant { bundle, grant_id } => {
            add_grant_to_bundle(&bundle, &grant_id).await
        }
        BundleSubcommand::FromProfile { profile, name, yes } => {
            from_profile(&profile, name.as_deref(), yes).await
        }
        BundleSubcommand::Remove { name, yes } => remove_bundle(&name, yes).await,
    }
}

async fn create_bundle(name: &str, description: Option<&str>) -> anyhow::Result<()> {
    let store = Store::connect(&current_database_url()).await?;

    if store.get_bundle_by_name(name).await?.is_some() {
        bail!("bundle with name '{name}' already exists");
    }

    let now = Utc::now();
    let bundle = Bundle {
        id: Uuid::new_v4(),
        name: name.to_string(),
        description: description.map(String::from),
        source_profile_id: None,
        created_at: now,
        updated_at: now,
    };

    store.insert_bundle(&bundle).await?;
    print_success(&format!(
        "Created bundle id={} name={}",
        bundle.id, bundle.name
    ))?;
    Ok(())
}

async fn list_bundles() -> anyhow::Result<()> {
    let store = Store::connect(&current_database_url()).await?;
    let bundles = store.list_bundles().await?;

    if bundles.is_empty() {
        print_success("No bundles found.")?;
        return Ok(());
    }

    for b in bundles {
        let source = b
            .source_profile_id
            .map(|id| format!(" (from profile {id})"))
            .unwrap_or_default();
        print_success(&format!("id={} name={}{source}", b.id, b.name))?;
    }
    Ok(())
}

async fn show_bundle(name: &str) -> anyhow::Result<()> {
    let store = Store::connect(&current_database_url()).await?;
    let bundle = store
        .get_bundle_by_name(name)
        .await?
        .with_context(|| format!("bundle not found: {name}"))?;

    print_success(&format!(
        "id={}\nname={}\ndescription={}\nsource_profile_id={}\ncreated_at={}\nupdated_at={}",
        bundle.id,
        bundle.name,
        bundle.description.as_deref().unwrap_or("(none)"),
        bundle
            .source_profile_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "(none)".to_string()),
        bundle.created_at.to_rfc3339(),
        bundle.updated_at.to_rfc3339(),
    ))?;

    let grants = store.list_grants_for_bundle(bundle.id).await?;
    if grants.is_empty() {
        print_success("\nNo grants in this bundle.")?;
    } else {
        print_success(&format!("\nGrants ({}):", grants.len()))?;
        for g in grants {
            print_success(&format!(
                "  id={} agent={} capability={} enabled={}",
                g.id, g.agent_name, g.capability_id, g.enabled
            ))?;
        }
    }
    Ok(())
}

async fn add_grant_to_bundle(bundle_name: &str, grant_id_str: &str) -> anyhow::Result<()> {
    let grant_id = Uuid::parse_str(grant_id_str)
        .with_context(|| format!("invalid grant id: {grant_id_str}"))?;
    let store = Store::connect(&current_database_url()).await?;

    let bundle = store
        .get_bundle_by_name(bundle_name)
        .await?
        .with_context(|| format!("bundle not found: {bundle_name}"))?;

    if store.get_grant(grant_id).await?.is_none() {
        bail!("grant not found: {grant_id_str}");
    }

    store.add_grant_to_bundle(bundle.id, grant_id).await?;
    print_success(&format!(
        "Added grant {grant_id_str} to bundle '{bundle_name}'"
    ))?;
    Ok(())
}

async fn from_profile(
    profile_name: &str,
    bundle_name: Option<&str>,
    yes: bool,
) -> anyhow::Result<()> {
    if !yes {
        bail!(
            "pass --yes to confirm: from-profile creates wildcard grants (agent='*', no expiry) \
             for every binding in profile '{profile_name}'"
        );
    }

    let store = Store::connect(&current_database_url()).await?;
    let profile = store
        .get_profile_by_name(profile_name)
        .await?
        .with_context(|| format!("profile not found: {profile_name}"))?;

    let target_name = bundle_name.unwrap_or(profile_name);
    if store.get_bundle_by_name(target_name).await?.is_some() {
        bail!("bundle with name '{target_name}' already exists");
    }

    let bindings = store.list_bindings_for_profile(profile.id).await?;
    if bindings.is_empty() {
        bail!("profile '{profile_name}' has no bindings to convert");
    }

    let policy = PolicyService::default();
    for binding in &bindings {
        let cred = store
            .get_credential(binding.credential_id)
            .await?
            .with_context(|| format!("credential not found: {}", binding.credential_id))?;
        policy.ensure_environment_allowed(&cred.environment)?;
    }

    let now = Utc::now();
    let bundle = Bundle {
        id: Uuid::new_v4(),
        name: target_name.to_string(),
        description: Some(format!("Converted from profile '{profile_name}'")),
        source_profile_id: Some(profile.id),
        created_at: now,
        updated_at: now,
    };
    store.insert_bundle(&bundle).await?;

    let mut grant_count = 0;
    for binding in &bindings {
        let connector_name = format!("{}-{}", binding.provider, binding.credential_id);
        let connector = if let Some(existing) = store.get_connector_by_name(&connector_name).await?
        {
            existing
        } else {
            let c = Connector {
                id: Uuid::new_v4(),
                name: connector_name.clone(),
                provider: binding.provider.clone(),
                credential_id: binding.credential_id,
                base_url: None,
                enabled: true,
                created_at: now,
                updated_at: now,
            };
            store.insert_connector(&c).await?;
            c
        };

        let cap_name = format!("{}.*", binding.provider);
        let capability = if let Some(existing) = store
            .get_capability_by_name(connector.id, &cap_name)
            .await?
        {
            existing
        } else {
            let c = Capability {
                id: Uuid::new_v4(),
                connector_id: connector.id,
                name: cap_name.clone(),
                description: Some(format!("Wildcard access to {}", binding.provider)),
                enabled: true,
                created_at: now,
            };
            store.insert_capability(&c).await?;
            c
        };

        let grant = Grant {
            id: Uuid::new_v4(),
            agent_name: "*".to_string(),
            capability_id: capability.id,
            ttl_minutes: None,
            max_requests: None,
            require_confirmation: false,
            enabled: true,
            created_at: now,
            expires_at: None,
        };
        store.insert_grant(&grant).await?;
        store.add_grant_to_bundle(bundle.id, grant.id).await?;
        grant_count += 1;
    }

    print_success(&format!(
        "Created bundle '{}' from profile '{}' with {} grant(s)",
        target_name, profile_name, grant_count
    ))?;
    Ok(())
}

async fn remove_bundle(name: &str, yes: bool) -> anyhow::Result<()> {
    if !yes {
        bail!("pass --yes to confirm deletion of bundle '{name}'");
    }

    let store = Store::connect(&current_database_url()).await?;
    let bundle = store
        .get_bundle_by_name(name)
        .await?
        .with_context(|| format!("bundle not found: {name}"))?;

    store.delete_bundle(bundle.id).await?;
    print_success(&format!("Removed bundle: {name}"))?;
    Ok(())
}
