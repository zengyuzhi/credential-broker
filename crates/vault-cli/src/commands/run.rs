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
use vault_secrets::{SecretStore, parse_secret_ref};

use crate::support::{
    config::current_database_url, keychain_migration::migrate_legacy_credentials_in_store,
    prompt::print_success,
};

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

/// Keywords in Keychain error messages that indicate an authorization/ACL failure.
const AUTH_ERROR_KEYWORDS: &[&str] = &[
    "User interaction is not allowed",
    "errAuthorizationDenied",
    "ACL add application",
    "errSecAuthFailed",
    "authorization denied",
    "not authorized",
    "access control",
];

/// Translate a raw Keychain error message into an actionable recovery message.
///
/// When the underlying error looks like an authorization/ACL problem, the returned
/// string explains concrete recovery steps.  For all other errors the original message
/// is returned unchanged so callers can still see the low-level cause.
pub(crate) fn explain_keychain_read_error(message: &str) -> String {
    let lower = message.to_lowercase();
    let is_auth_error = AUTH_ERROR_KEYWORDS
        .iter()
        .any(|kw| lower.contains(&kw.to_lowercase()));

    if is_auth_error {
        let exe_hint = std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "the current vault-cli binary".to_string());
        format!(
            "Keychain access for this credential is not authorized for the current vault-cli binary.\n\
             Re-add the credential with the updated CLI, or manually allow {exe_hint} \
             in Keychain Access for this item.\n\
             Use VAULT_TRUSTED_APP_PATHS only for recovery/debugging.\n\
             Underlying cause: {message}"
        )
    } else {
        message.to_string()
    }
}

#[derive(Debug, Args)]
#[command(
    about = "Launch a command with profile credentials via env injection (compatibility path)"
)]
#[command(
    long_about = "Use this when a tool still expects env injection into the child-process \
    environment. `vault run` remains supported for compatibility, but brokered access through \
    the local vault is the preferred path when a tool can use it."
)]
pub struct RunCommand {
    #[arg(long, help = "Profile name whose bindings supply credentials")]
    pub profile: String,
    #[arg(
        long,
        default_value = "unknown-agent",
        help = "Agent identifier recorded in lease and telemetry"
    )]
    pub agent: String,
    #[arg(long, help = "Optional project name injected as VAULT_PROJECT")]
    pub project: Option<String>,
    #[arg(
        last = true,
        required = true,
        help = "Command and arguments to execute"
    )]
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
    let _ = migrate_legacy_credentials_in_store(&store).await?;
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

    // Capture first credential_id for the launch audit event (FK requires a real credential).
    let first_credential_id = bindings[0].credential_id;
    let resolved = resolve_bound_credentials(&store, bindings).await?;
    debug_log(format!("resolved {} bound credentials", resolved.len()));
    let mut env_map = resolve_env_for_profile(resolved)?;
    debug_log(format!(
        "resolved env keys: {:?}",
        env_map.keys().collect::<Vec<_>>()
    ));
    // TTL is a compile-time constant for `vault run`; `NonZeroU32::new(60)`
    // is statically known to be `Some`, so unwrap is infallible here.
    let ttl = std::num::NonZeroU32::new(60).expect("60 is nonzero");
    let (lease, raw_token) = issue_lease(profile.id, &cmd.agent, cmd.project.clone(), ttl);
    store.insert_lease(&lease).await?;
    debug_log(format!(
        "issued lease id={} token_len={}",
        lease.id,
        raw_token.len()
    ));
    env_map.insert("VAULT_PROFILE".to_string(), cmd.profile.clone());
    env_map.insert("VAULT_AGENT".to_string(), cmd.agent.clone());
    // Copy the lease token into the env map so the primary `Zeroizing<String>`
    // binding wipes at end of scope; the copy in `env_map` is consumed by
    // `Command::envs` below and not persisted.
    env_map.insert("VAULT_LEASE_TOKEN".to_string(), (*raw_token).clone());
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

    // Record a launch usage event for audit and stats.
    {
        let telemetry = vault_telemetry::writer::TelemetryWriter::new(store.clone());
        let launch_event = vault_core::models::UsageEvent {
            id: uuid::Uuid::new_v4(),
            provider: "vault".to_string(),
            credential_id: first_credential_id,
            lease_id: Some(lease.id),
            agent_name: cmd.agent.clone(),
            project: cmd.project.clone(),
            mode: vault_core::models::AccessMode::Inject,
            operation: "process_launch".to_string(),
            endpoint: None,
            model: None,
            request_count: 1,
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
            estimated_cost_micros: None,
            status_code: status.code().map(|c| c as i64),
            success: status.success(),
            latency_ms: 0,
            error_text: if status.success() {
                None
            } else {
                Some(format!("exit {status}"))
            },
            created_at: chrono::Utc::now(),
        };
        if let Err(err) = telemetry.write_usage_event(&launch_event).await {
            debug_log(format!("failed to record launch event: {err}"));
        }
    }

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
    let keychain = vault_secrets::MacOsKeychainStore;

    let mut resolved = Vec::new();
    for binding in bindings {
        if matches!(binding.mode, AccessMode::Proxy) {
            continue;
        }

        let credential = store
            .get_credential(binding.credential_id)
            .await?
            .with_context(|| format!("missing credential for binding {}", binding.id))?;
        let (service, account) = parse_secret_ref(&credential.secret_ref)?;
        debug_log(format!(
            "resolving binding id={} provider={} credential_id={} mode={:?} service={} account={}",
            binding.id, binding.provider, credential.id, binding.mode, service, account
        ));

        #[cfg(target_os = "macos")]
        let secret_value = {
            debug_log(format!(
                "reading macOS keychain service={} account={}",
                service, account
            ));
            let value = keychain.get(service, account).await.map_err(|err| {
                let raw = err.to_string();
                let enhanced = explain_keychain_read_error(&raw);
                if enhanced != raw {
                    anyhow::anyhow!("{}", enhanced)
                } else {
                    err
                }
            })?;
            debug_log(format!(
                "loaded keychain secret for credential_id={}",
                credential.id,
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
                // Copy the inner String out of `Zeroizing<String>` into the
                // HashMap. The source `secret_value` is wiped when dropped at
                // end of scope; the copy in `fields` is wiped by
                // `ResolvedCredential`'s custom Drop impl. Audit ZA-0002.
                fields: HashMap::from([(field_name.to_string(), (*secret_value).clone())]),
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

    use super::{explain_keychain_read_error, resolve_env_for_profile};

    // --- explain_keychain_read_error tests ---

    #[test]
    fn explain_keychain_read_error_auth_failure_mentions_re_add() {
        let msg = explain_keychain_read_error("User interaction is not allowed");
        assert!(
            msg.contains("re-add") || msg.contains("Re-add"),
            "expected re-add guidance, got: {msg}"
        );
    }

    #[test]
    fn explain_keychain_read_error_auth_failure_mentions_binary_path() {
        let msg = explain_keychain_read_error("errAuthorizationDenied");
        assert!(
            msg.contains("vault-cli"),
            "expected vault-cli binary mention, got: {msg}"
        );
    }

    #[test]
    fn explain_keychain_read_error_auth_failure_mentions_trusted_app_paths() {
        let msg = explain_keychain_read_error("ACL add application");
        assert!(
            msg.contains("VAULT_TRUSTED_APP_PATHS"),
            "expected VAULT_TRUSTED_APP_PATHS mention, got: {msg}"
        );
    }

    #[test]
    fn explain_keychain_read_error_non_auth_error_passes_through() {
        let original = "The specified item could not be found in the keychain";
        let msg = explain_keychain_read_error(original);
        // Non-auth errors should still contain the original message
        assert!(
            msg.contains(original),
            "expected original message preserved, got: {msg}"
        );
    }

    #[test]
    fn explain_keychain_read_error_auth_failure_preserves_original_cause() {
        let original = "User interaction is not allowed";
        let msg = explain_keychain_read_error(original);
        assert!(
            msg.contains(original),
            "expected original cause preserved in output, got: {msg}"
        );
    }

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
