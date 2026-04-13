use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use chrono::Utc;
use clap::{Args, Subcommand};
use uuid::Uuid;
use vault_core::models::{Credential, CredentialKind};
use vault_db::Store;
use vault_providers::schema_for;
use vault_secrets::{KEYCHAIN_SERVICE_NAME, SecretStore};

use crate::support::{
    config::current_database_url,
    prompt::{print_success, prompt_secret},
};

#[derive(Debug, Args)]
#[command(about = "Add, list, enable, disable, or remove stored credentials")]
pub struct CredentialCommand {
    #[command(subcommand)]
    pub command: CredentialSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum CredentialSubcommand {
    #[command(about = "Store a new credential in macOS Keychain")]
    Add {
        #[arg(help = "Provider name (e.g. openai, anthropic, twitterapi)")]
        provider: String,
        #[arg(help = "Human-readable label for this credential")]
        label: String,
        #[arg(long, default_value = "api_key", help = "Credential type: api_key, bearer_token, oauth, bundle")]
        kind: String,
        #[arg(long, default_value = "work", help = "Environment tag: work, personal, prod")]
        env: String,
    },
    #[command(about = "List all stored credentials")]
    List,
    #[command(about = "Disable a credential (prevents use in profiles)")]
    Disable {
        #[arg(help = "Credential UUID to disable")]
        id: String,
    },
    #[command(about = "Re-enable a previously disabled credential")]
    Enable {
        #[arg(help = "Credential UUID to enable")]
        id: String,
    },
    #[command(about = "Permanently remove a credential and its Keychain secret")]
    Remove {
        #[arg(help = "Credential UUID to remove")]
        id: String,
        #[arg(long, help = "Skip confirmation prompt")]
        yes: bool,
    },
}

pub async fn run_credential_command(cmd: CredentialCommand) -> anyhow::Result<()> {
    match cmd.command {
        CredentialSubcommand::Add {
            provider,
            label,
            kind,
            env,
        } => add_credential(&provider, &label, &kind, &env).await,
        CredentialSubcommand::List => list_credentials().await,
        CredentialSubcommand::Disable { id } => set_credential_enabled(&id, false).await,
        CredentialSubcommand::Enable { id } => set_credential_enabled(&id, true).await,
        CredentialSubcommand::Remove { id, yes } => remove_credential(&id, yes).await,
    }
}

async fn add_credential(
    provider: &str,
    label: &str,
    kind: &str,
    environment: &str,
) -> anyhow::Result<()> {
    let schema =
        schema_for(provider).with_context(|| format!("unsupported provider schema: {provider}"))?;
    let credential_kind = parse_credential_kind(kind)?;

    if schema.required_fields.len() != 1 {
        bail!(
            "provider {provider} requires {} fields, but multi-field storage is not implemented yet",
            schema.required_fields.len()
        );
    }

    let store = Store::connect(&current_database_url()).await?;
    let credential_id = Uuid::new_v4();
    let field_name = schema.required_fields[0];
    let secret_value = prompt_secret(field_name)?;

    if secret_value.trim().is_empty() {
        bail!("secret field {field_name} cannot be empty");
    }

    #[cfg(target_os = "macos")]
    let secret_ref = {
        let keychain = vault_secrets::MacOsKeychainStore;
        let account = build_keychain_account(credential_id, field_name);
        let current_exe = std::env::current_exe()?;
        let env_override = std::env::var("VAULT_TRUSTED_APP_PATHS").ok();
        let trusted_apps = keychain_account_and_access_targets(
            credential_id,
            field_name,
            &current_exe,
            env_override.as_deref(),
        )?
        .1;
        keychain
            .put_with_access(
                KEYCHAIN_SERVICE_NAME,
                &account,
                &secret_value,
                &trusted_apps,
            )
            .await?
    };

    #[cfg(not(target_os = "macos"))]
    let secret_ref = {
        bail!("credential add is only implemented for macOS in Phase 1");
    };

    let now = Utc::now();
    let credential = Credential {
        id: credential_id,
        provider: provider.to_string(),
        kind: credential_kind,
        label: label.to_string(),
        secret_ref,
        environment: environment.to_string(),
        owner: None,
        enabled: true,
        created_at: now,
        updated_at: now,
        last_used_at: None,
    };

    store.insert_credential(&credential).await?;
    print_success(&format!(
        "Added credential id={} provider={} label={} environment={}",
        credential.id, credential.provider, credential.label, credential.environment
    ))?;
    Ok(())
}

async fn list_credentials() -> anyhow::Result<()> {
    let store = Store::connect(&current_database_url()).await?;
    let credentials = store.list_credentials().await?;

    if credentials.is_empty() {
        print_success("No credentials stored.")?;
        return Ok(());
    }

    for credential in credentials {
        print_success(&format!(
            "id={} provider={} label={} env={} enabled={} last_used_at={}",
            credential.id,
            credential.provider,
            credential.label,
            credential.environment,
            credential.enabled,
            credential
                .last_used_at
                .map(|value| value.to_rfc3339())
                .unwrap_or_else(|| "never".to_string())
        ))?;
    }

    Ok(())
}

async fn set_credential_enabled(id: &str, enabled: bool) -> anyhow::Result<()> {
    let credential_id =
        Uuid::parse_str(id).with_context(|| format!("invalid credential id: {id}"))?;
    let store = Store::connect(&current_database_url()).await?;
    let existing = store.get_credential(credential_id).await?;

    if existing.is_none() {
        bail!("credential not found: {id}");
    }

    store.set_credential_enabled(credential_id, enabled).await?;
    print_success(&format!(
        "Credential {} {}.",
        credential_id,
        if enabled { "enabled" } else { "disabled" }
    ))?;
    Ok(())
}

async fn remove_credential(id: &str, yes: bool) -> anyhow::Result<()> {
    if !yes {
        bail!("refusing to remove credential without --yes");
    }

    let credential_id =
        Uuid::parse_str(id).with_context(|| format!("invalid credential id: {id}"))?;
    let store = Store::connect(&current_database_url()).await?;
    let credential = store
        .get_credential(credential_id)
        .await?
        .with_context(|| format!("credential not found: {id}"))?;

    let (service, account) = parse_secret_ref(&credential.secret_ref)?;

    #[cfg(target_os = "macos")]
    {
        let keychain = vault_secrets::MacOsKeychainStore;
        keychain.delete(&service, &account).await?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        bail!("credential remove is only implemented for macOS in Phase 1");
    }

    store.delete_credential(credential_id).await?;
    print_success(&format!("Removed credential {}.", credential_id))?;
    Ok(())
}

fn parse_credential_kind(value: &str) -> anyhow::Result<CredentialKind> {
    match value {
        "api_key" => Ok(CredentialKind::ApiKey),
        "bearer_token" => Ok(CredentialKind::BearerToken),
        "oauth" => Ok(CredentialKind::OAuth),
        "bundle" => Ok(CredentialKind::Bundle),
        other => bail!("unsupported credential kind: {other}"),
    }
}

fn build_keychain_account(credential_id: Uuid, field_name: &str) -> String {
    format!("credential:{credential_id}:{field_name}")
}

/// Compute the Keychain account string and the list of trusted application paths for a
/// given credential field.
///
/// Extracted as a pure helper so it can be unit-tested without touching the live Keychain.
///
/// # Parameters
/// - `credential_id` – UUID of the credential being stored.
/// - `field_name`    – schema field being stored (e.g. `"api_key"`).
/// - `current_exe`   – path to the running executable (used as the primary trusted app).
/// - `env_override`  – optional value of `VAULT_TRUSTED_APP_PATHS` (colon-separated).
///
/// # Returns
/// A tuple `(account, trusted_paths)`.
fn keychain_account_and_access_targets(
    credential_id: Uuid,
    field_name: &str,
    current_exe: &Path,
    env_override: Option<&str>,
) -> anyhow::Result<(String, Vec<PathBuf>)> {
    let account = build_keychain_account(credential_id, field_name);
    let trusted_apps = vault_secrets::trusted_application_paths_for(current_exe, env_override);
    anyhow::ensure!(
        !trusted_apps.is_empty(),
        "could not resolve any trusted application paths for Keychain ACL"
    );
    Ok((account, trusted_apps))
}

fn parse_secret_ref(secret_ref: &str) -> anyhow::Result<(String, String)> {
    let (service, account) = secret_ref
        .split_once(':')
        .with_context(|| format!("invalid secret ref: {secret_ref}"))?;
    Ok((service.to_string(), account.to_string()))
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use tempfile::TempDir;
    use uuid::Uuid;
    use vault_core::models::{Credential, CredentialKind};
    use vault_db::Store;

    use super::{
        build_keychain_account, keychain_account_and_access_targets, parse_credential_kind,
        parse_secret_ref, set_credential_enabled,
    };
    use crate::support::config::{
        clear_test_database_url, current_database_url, set_test_database_url, test_database_lock,
    };

    fn setup_test_db() -> TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_url = format!("sqlite:{}", dir.path().join("credentials.db").display());
        set_test_database_url(db_url);
        dir
    }

    #[test]
    fn keychain_account_name_should_include_credential_id_and_field() {
        let id = Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").expect("uuid");
        let account = build_keychain_account(id, "api_key");
        assert_eq!(
            account,
            "credential:123e4567-e89b-12d3-a456-426614174000:api_key"
        );
    }

    /// `keychain_account_and_access_targets` returns the correct account string and a
    /// non-empty trusted-path list that includes the supplied exe path.
    #[test]
    fn keychain_account_and_access_targets_returns_correct_account_and_paths() {
        use std::path::Path;
        let id = Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").expect("uuid");
        let fake_exe = Path::new("/nonexistent/target/debug/vault-cli");

        let (account, paths) = keychain_account_and_access_targets(id, "api_key", fake_exe, None)
            .expect("helper should succeed");

        // Account must follow the established convention.
        assert_eq!(
            account,
            "credential:123e4567-e89b-12d3-a456-426614174000:api_key"
        );

        // The trusted-path list must contain the resolved vault-cli path.
        assert!(!paths.is_empty(), "trusted paths must not be empty");
        assert!(
            paths.iter().any(|p| p.ends_with("vault-cli")),
            "expected vault-cli in trusted paths, got: {paths:?}"
        );
    }

    /// env_override paths are included in the trusted-path list.
    #[test]
    fn keychain_account_and_access_targets_includes_env_override_paths() {
        use std::path::Path;
        let id = Uuid::new_v4();
        let fake_exe = Path::new("/nonexistent/target/debug/vault-cli");
        let extra = "/nonexistent/other/bin/helper";

        let (_account, paths) =
            keychain_account_and_access_targets(id, "api_key", fake_exe, Some(extra))
                .expect("helper should succeed");

        assert!(
            paths.iter().any(|p| p.ends_with("helper")),
            "expected helper in trusted paths, got: {paths:?}"
        );
    }

    #[test]
    fn parse_credential_kind_should_support_api_key() {
        let kind = parse_credential_kind("api_key").expect("parse kind");
        assert!(matches!(kind, CredentialKind::ApiKey));
    }

    #[test]
    fn parse_secret_ref_should_split_service_and_account() {
        let (service, account) = parse_secret_ref(
            "ai.zyr1.vault:credential:123e4567-e89b-12d3-a456-426614174000:api_key",
        )
        .expect("parse secret ref");
        assert_eq!(service, "ai.zyr1.vault");
        assert_eq!(
            account,
            "credential:123e4567-e89b-12d3-a456-426614174000:api_key"
        );
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn set_credential_enabled_should_use_current_database_url_override() {
        let _guard = test_database_lock().lock().expect("test lock");
        let _dir = setup_test_db();

        let store = Store::connect(&current_database_url())
            .await
            .expect("connect store");
        let credential = Credential {
            id: Uuid::new_v4(),
            provider: "openai".to_string(),
            kind: CredentialKind::ApiKey,
            label: "work-main".to_string(),
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

        let result = set_credential_enabled(&credential.id.to_string(), false).await;
        let updated = store
            .get_credential(credential.id)
            .await
            .expect("load updated credential")
            .expect("credential present");
        clear_test_database_url();

        result.expect("disable credential");
        assert!(!updated.enabled);
    }
}
