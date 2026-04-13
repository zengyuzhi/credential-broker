use std::{collections::HashMap, process::Stdio};

use anyhow::{Context, bail};
use clap::Args;
use tokio::process::Command;
use vault_core::{
    models::AccessMode,
    provider::{ProviderAdapter, ResolvedCredential},
};
use vault_db::Store;
use vault_policy::lease::issue_lease;
use vault_providers::adapter_for;
use vault_secrets::{KEYCHAIN_SERVICE_NAME, SecretStore};

use crate::support::{config::current_database_url, prompt::print_success};

fn debug_enabled() -> bool {
    matches!(
        std::env::var("VAULT_DEBUG_RUN").as_deref(),
        Ok("1") | Ok("true") | Ok("yes") | Ok("on")
    )
}

fn debug_log(message: impl AsRef<str>) {
    if debug_enabled() {
        eprintln!("[vault-run-debug] {}", message.as_ref());
    }
}

#[derive(Debug, Args)]
pub struct RunCommand {
    #[arg(long)]
    pub profile: String,
    #[arg(long, default_value = "unknown-agent")]
    pub agent: String,
    #[arg(long)]
    pub project: Option<String>,
    #[arg(last = true, required = true)]
    pub command: Vec<String>,
}

pub async fn run_agent_command(cmd: RunCommand) -> anyhow::Result<()> {
    let database_url = current_database_url();
    debug_log(format!(
        "starting run profile={} agent={} project={:?} cwd={} db_url={}",
        cmd.profile,
        cmd.agent,
        cmd.project,
        std::env::current_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| "<unknown>".to_string()),
        database_url
    ));

    let store = Store::connect(&database_url).await?;
    let profile = store
        .get_profile_by_name(&cmd.profile)
        .await?
        .with_context(|| format!("profile not found: {}", cmd.profile))?;
    debug_log(format!(
        "loaded profile id={} name={}",
        profile.id, profile.name
    ));
    let bindings = store.list_bindings_for_profile(profile.id).await?;
    debug_log(format!(
        "loaded {} bindings for profile {}",
        bindings.len(),
        profile.id
    ));
    if bindings.is_empty() {
        bail!("profile {} has no bindings", cmd.profile);
    }

    let resolved = resolve_bound_credentials(&store, bindings).await?;
    debug_log(format!("resolved {} bound credentials", resolved.len()));
    let mut env_map = resolve_env_for_profile(resolved)?;
    debug_log(format!(
        "resolved env keys: {:?}",
        env_map.keys().collect::<Vec<_>>()
    ));
    let (lease, raw_token) = issue_lease(profile.id, &cmd.agent, cmd.project.clone(), 60);
    store.insert_lease(&lease).await?;
    debug_log(format!(
        "issued lease id={} token_len={}",
        lease.id,
        raw_token.len()
    ));
    env_map.insert("VAULT_PROFILE".to_string(), cmd.profile.clone());
    env_map.insert("VAULT_AGENT".to_string(), cmd.agent.clone());
    env_map.insert("VAULT_LEASE_TOKEN".to_string(), raw_token);
    if let Some(project) = &cmd.project {
        env_map.insert("VAULT_PROJECT".to_string(), project.clone());
    }

    let program = cmd
        .command
        .first()
        .cloned()
        .context("missing command program")?;
    let args = cmd.command.iter().skip(1).cloned().collect::<Vec<_>>();
    debug_log(format!("spawning program={} args={:?}", program, args));

    let status = Command::new(&program)
        .args(&args)
        .envs(&env_map)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .with_context(|| format!("failed to spawn command: {program}"))?;
    debug_log(format!("child exited with status {}", status));

    if !status.success() {
        bail!("command exited with status {status}");
    }

    print_success(&format!(
        "Ran command with profile={} agent={} bindings={}",
        cmd.profile,
        cmd.agent,
        env_map.len()
    ))?;
    Ok(())
}

async fn resolve_bound_credentials(
    store: &Store,
    bindings: Vec<vault_core::models::ProfileBinding>,
) -> anyhow::Result<Vec<(String, AccessMode, ResolvedCredential)>> {
    #[cfg(target_os = "macos")]
    let keychain = vault_secrets::MacOsKeychainStore::default();

    let mut resolved = Vec::new();
    for binding in bindings {
        if matches!(binding.mode, AccessMode::Proxy) {
            continue;
        }

        let credential = store
            .get_credential(binding.credential_id)
            .await?
            .with_context(|| format!("missing credential for binding {}", binding.id))?;
        let (_service, account) = parse_secret_ref(&credential.secret_ref)?;
        debug_log(format!(
            "resolving binding id={} provider={} credential_id={} mode={:?} account={}",
            binding.id, binding.provider, credential.id, binding.mode, account
        ));

        #[cfg(target_os = "macos")]
        let secret_value = {
            debug_log(format!(
                "reading macOS keychain service={} account={}",
                KEYCHAIN_SERVICE_NAME, account
            ));
            let value = keychain.get(KEYCHAIN_SERVICE_NAME, &account).await?;
            debug_log(format!(
                "loaded keychain secret for credential_id={} (len={})",
                credential.id,
                value.len()
            ));
            value
        };

        #[cfg(not(target_os = "macos"))]
        let secret_value = {
            bail!("vault run is only implemented for macOS in Phase 1");
        };

        let field_name = infer_field_name(&credential.provider);
        resolved.push((
            credential.provider.clone(),
            binding.mode,
            ResolvedCredential {
                provider: credential.provider,
                label: credential.label,
                fields: HashMap::from([(field_name.to_string(), secret_value)]),
            },
        ));
    }

    Ok(resolved)
}

fn infer_field_name(provider: &str) -> &'static str {
    match provider {
        "github" => "token",
        _ => "api_key",
    }
}

fn parse_secret_ref(secret_ref: &str) -> anyhow::Result<(String, String)> {
    let (service, account) = secret_ref
        .split_once(':')
        .with_context(|| format!("invalid secret ref: {secret_ref}"))?;
    Ok((service.to_string(), account.to_string()))
}

fn resolve_env_for_profile(
    bindings: Vec<(String, AccessMode, ResolvedCredential)>,
) -> anyhow::Result<HashMap<String, String>> {
    let mut env_map = HashMap::new();

    for (provider, mode, credential) in bindings {
        if matches!(mode, AccessMode::Proxy) {
            continue;
        }
        let adapter = adapter_for(&provider)?;
        merge_env_map(&mut env_map, adapter.as_ref(), &credential)?;
    }

    Ok(env_map)
}

fn merge_env_map(
    env_map: &mut HashMap<String, String>,
    adapter: &dyn ProviderAdapter,
    credential: &ResolvedCredential,
) -> anyhow::Result<()> {
    for (key, value) in adapter.env_map(credential)? {
        env_map.insert(key, value);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use vault_core::{models::AccessMode, provider::ResolvedCredential};

    use super::resolve_env_for_profile;

    #[test]
    fn resolve_env_for_profile_should_map_bound_provider_keys() {
        let bindings = vec![(
            "openai".to_string(),
            AccessMode::Inject,
            ResolvedCredential {
                provider: "openai".to_string(),
                label: "work-main".to_string(),
                fields: HashMap::from([("api_key".to_string(), "secret-openai".to_string())]),
            },
        )];

        let env = resolve_env_for_profile(bindings).expect("resolve env");
        assert_eq!(
            env.get("OPENAI_API_KEY").map(String::as_str),
            Some("secret-openai")
        );
    }
}
