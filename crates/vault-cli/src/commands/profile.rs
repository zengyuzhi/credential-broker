use anyhow::{Context, bail};
use chrono::Utc;
use clap::{Args, Subcommand};
use vault_core::models::{AccessMode, Profile, ProfileBinding};
use vault_db::Store;
use vault_policy::service::PolicyService;
use vault_providers::schema_for;

use crate::support::{config::current_database_url, prompt::print_success};

#[derive(Debug, Args)]
pub struct ProfileCommand {
    #[command(subcommand)]
    pub command: ProfileSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ProfileSubcommand {
    Create {
        name: String,
    },
    List,
    Bind {
        profile: String,
        provider: String,
        credential_id: String,
        #[arg(long, default_value = "either")]
        mode: String,
    },
    Show {
        name: String,
    },
}

pub async fn run_profile_command(cmd: ProfileCommand) -> anyhow::Result<()> {
    match cmd.command {
        ProfileSubcommand::Create { name } => {
            let output = create_profile(&name).await?;
            print_success(&output)
        }
        ProfileSubcommand::List => {
            let output = list_profiles().await?;
            print_success(&output)
        }
        ProfileSubcommand::Bind {
            profile,
            provider,
            credential_id,
            mode,
        } => {
            let output = bind_profile(&profile, &provider, &credential_id, &mode).await?;
            print_success(&output)
        }
        ProfileSubcommand::Show { name } => {
            let output = show_profile(&name).await?;
            print_success(&output)
        }
    }
}

pub async fn create_profile(name: &str) -> anyhow::Result<String> {
    if name.trim().is_empty() {
        bail!("profile name cannot be empty");
    }

    let store = Store::connect(&current_database_url()).await?;
    if store.get_profile_by_name(name).await?.is_some() {
        bail!("profile already exists: {name}");
    }

    let profile = Profile {
        id: uuid::Uuid::new_v4(),
        name: name.to_string(),
        description: None,
        default_project: None,
        created_at: Utc::now(),
    };

    store.insert_profile(&profile).await?;
    Ok(format!(
        "Created profile name={} id={}",
        profile.name, profile.id
    ))
}

pub async fn list_profiles() -> anyhow::Result<String> {
    let store = Store::connect(&current_database_url()).await?;
    let profiles = store.list_profiles().await?;

    if profiles.is_empty() {
        return Ok("No profiles created.".to_string());
    }

    Ok(profiles
        .into_iter()
        .map(|profile| format_profile_summary(&profile))
        .collect::<Vec<_>>()
        .join("\n"))
}

pub async fn bind_profile(
    profile_name: &str,
    provider: &str,
    credential_id: &str,
    mode: &str,
) -> anyhow::Result<String> {
    let access_mode = parse_access_mode(mode)?;
    let schema =
        schema_for(provider).with_context(|| format!("unsupported provider schema: {provider}"))?;
    let store = Store::connect(&current_database_url()).await?;
    let profile = store
        .get_profile_by_name(profile_name)
        .await?
        .with_context(|| format!("profile not found: {profile_name}"))?;
    let credential_uuid = uuid::Uuid::parse_str(credential_id)
        .with_context(|| format!("invalid credential id: {credential_id}"))?;
    let credential = store
        .get_credential(credential_uuid)
        .await?
        .with_context(|| format!("credential not found: {credential_id}"))?;

    if credential.provider != provider {
        bail!(
            "credential provider mismatch: credential is {}, binding requested {}",
            credential.provider,
            provider
        );
    }
    if !credential.enabled {
        bail!("credential is disabled: {credential_id}");
    }

    let policy = PolicyService::default();
    policy.ensure_environment_allowed(&credential.environment)?;

    if matches!(access_mode, AccessMode::Proxy) {
        bail!("proxy bindings are not enabled in Phase 1");
    }
    if schema.default_mode == AccessMode::Inject && matches!(access_mode, AccessMode::Either) {
        // allowed, just more permissive than the provider default
    }

    let binding = ProfileBinding {
        id: uuid::Uuid::new_v4(),
        profile_id: profile.id,
        provider: provider.to_string(),
        credential_id: credential.id,
        mode: access_mode,
    };

    store.insert_binding(&binding).await?;
    Ok(format!(
        "Bound profile={} provider={} credential_id={} mode={:?}",
        profile.name, binding.provider, binding.credential_id, binding.mode
    ))
}

pub async fn show_profile(name: &str) -> anyhow::Result<String> {
    let store = Store::connect(&current_database_url()).await?;
    let profile = store
        .get_profile_by_name(name)
        .await?
        .with_context(|| format!("profile not found: {name}"))?;
    let bindings = store.list_bindings_for_profile(profile.id).await?;

    let mut lines = vec![format_profile_detail(&profile)];
    if bindings.is_empty() {
        lines.push("bindings=none".to_string());
    } else {
        lines.extend(bindings.into_iter().map(|binding| {
            format!(
                "binding provider={} credential_id={} mode={:?}",
                binding.provider, binding.credential_id, binding.mode
            )
        }));
    }

    Ok(lines.join("\n"))
}

fn parse_access_mode(value: &str) -> anyhow::Result<AccessMode> {
    match value {
        "inject" => Ok(AccessMode::Inject),
        "either" => Ok(AccessMode::Either),
        "proxy" => Ok(AccessMode::Proxy),
        other => bail!("unsupported access mode: {other}"),
    }
}

fn format_profile_summary(profile: &Profile) -> String {
    format!(
        "id={} name={} default_project={} created_at={}",
        profile.id,
        profile.name,
        profile
            .default_project
            .clone()
            .unwrap_or_else(|| "none".to_string()),
        profile.created_at.to_rfc3339()
    )
}

fn format_profile_detail(profile: &Profile) -> String {
    format!(
        "id={} name={} description={} default_project={} created_at={}",
        profile.id,
        profile.name,
        profile
            .description
            .clone()
            .unwrap_or_else(|| "none".to_string()),
        profile
            .default_project
            .clone()
            .unwrap_or_else(|| "none".to_string()),
        profile.created_at.to_rfc3339()
    )
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use tempfile::TempDir;
    use vault_core::models::{Credential, CredentialKind};
    use vault_db::Store;

    use super::{bind_profile, create_profile, list_profiles, show_profile};
    use crate::support::config::{
        clear_test_database_url, current_database_url, set_test_database_url, test_database_lock,
    };

    fn setup_test_db() -> TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_url = format!("sqlite:{}", dir.path().join("profiles.db").display());
        set_test_database_url(db_url);
        dir
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn create_profile_should_persist_and_show() {
        let _guard = test_database_lock().lock().expect("test lock");
        let _dir = setup_test_db();

        create_profile("coding").await.expect("create profile");
        let output = show_profile("coding").await.expect("show profile");

        assert!(output.contains("name=coding"));
        assert!(output.contains("bindings=none"));
        clear_test_database_url();
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn list_profiles_should_include_created_profile() {
        let _guard = test_database_lock().lock().expect("test lock");
        let _dir = setup_test_db();

        create_profile("alpha").await.expect("create alpha");
        create_profile("zed").await.expect("create zed");
        let output = list_profiles().await.expect("list profiles");

        assert!(output.contains("name=alpha"));
        assert!(output.contains("name=zed"));
        clear_test_database_url();
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn bind_profile_should_show_binding_details() {
        let _guard = test_database_lock().lock().expect("test lock");
        let _dir = setup_test_db();

        create_profile("coding").await.expect("create profile");
        let store = Store::connect(&current_database_url())
            .await
            .expect("connect store");
        let credential = Credential {
            id: uuid::Uuid::new_v4(),
            provider: "twitterapi".to_string(),
            kind: CredentialKind::ApiKey,
            label: "social-main".to_string(),
            secret_ref: "ai.zyr1.vault:credential:test:api_key".to_string(),
            environment: "work".to_string(),
            owner: None,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_used_at: None,
        };
        store
            .insert_credential(&credential)
            .await
            .expect("insert credential");

        bind_profile("coding", "twitterapi", &credential.id.to_string(), "inject")
            .await
            .expect("bind profile");
        let output = show_profile("coding").await.expect("show profile");

        assert!(output.contains("binding provider=twitterapi"));
        assert!(output.contains("mode=Inject"));
        clear_test_database_url();
    }
}
