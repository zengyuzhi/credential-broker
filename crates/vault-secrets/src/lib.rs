use std::path::Path;

use anyhow::Context;
use async_trait::async_trait;
use zeroize::Zeroizing;

mod access;
pub use access::trusted_application_paths_for;

pub const KEYCHAIN_SERVICE_NAME: &str = "dev.credential-broker.vault";
pub const LEGACY_KEYCHAIN_SERVICE_NAME: &str = "ai.zyr1.vault";

/// Parse the persisted `secret_ref` format `"service:account"`.
///
/// The account component may itself contain additional `:` separators, so only the
/// first separator is treated as the service/account boundary.
pub fn parse_secret_ref(secret_ref: &str) -> anyhow::Result<(&str, &str)> {
    secret_ref
        .split_once(':')
        .ok_or_else(|| anyhow::anyhow!("invalid secret ref: {secret_ref}"))
}

#[cfg(target_os = "macos")]
pub async fn migrate_legacy_secret_ref(
    secret_ref: &str,
    current_exe: &Path,
    env_override: Option<&str>,
) -> anyhow::Result<Option<String>> {
    let (service, account) = parse_secret_ref(secret_ref)?;
    if service != LEGACY_KEYCHAIN_SERVICE_NAME {
        return Ok(None);
    }

    let trusted_apps = trusted_application_paths_for(current_exe, env_override);
    anyhow::ensure!(
        !trusted_apps.is_empty(),
        "could not resolve any trusted application paths for Keychain ACL"
    );

    let store = MacOsKeychainStore;
    let secret = store
        .get(service, account)
        .await
        .with_context(|| format!("failed to read legacy keychain item {service}/{account}"))?;
    let new_ref = store
        .put_with_access(
            KEYCHAIN_SERVICE_NAME,
            account,
            secret.as_str(),
            &trusted_apps,
        )
        .await
        .with_context(|| {
            format!("failed to write migrated keychain item {KEYCHAIN_SERVICE_NAME}/{account}")
        })?;

    if let Err(err) = store.delete(service, account).await {
        let _ = store.delete(KEYCHAIN_SERVICE_NAME, account).await;
        return Err(err).with_context(|| {
            format!("failed to delete legacy keychain item {service}/{account} after migration")
        });
    }

    Ok(Some(new_ref))
}

/// Read/delete interface for secret storage.
///
/// Writes are intentionally **not** on this trait — the only supported write
/// path is `MacOsKeychainStore::put_with_access`, which requires explicit
/// trusted-application ACLs. A plain `put` (no ACL) was previously exposed
/// here; it created a dangerous default where the easy path stored secrets
/// that any application could read. Removed per audit finding SE-04.
///
/// `get` returns `Zeroizing<String>` so the heap buffer is wiped on drop —
/// audit ZA-0001.
#[async_trait]
pub trait SecretStore: Send + Sync {
    async fn get(&self, service: &str, account: &str) -> anyhow::Result<Zeroizing<String>>;
    async fn delete(&self, service: &str, account: &str) -> anyhow::Result<()>;
}

#[cfg(target_os = "macos")]
pub mod keychain;

#[cfg(target_os = "macos")]
pub use keychain::MacOsKeychainStore;

#[cfg(test)]
mod tests {
    use super::{KEYCHAIN_SERVICE_NAME, LEGACY_KEYCHAIN_SERVICE_NAME, parse_secret_ref};

    #[test]
    fn keychain_service_name_should_use_generic_product_namespace() {
        assert_eq!(KEYCHAIN_SERVICE_NAME, "dev.credential-broker.vault");
    }

    #[test]
    fn parse_secret_ref_should_preserve_legacy_service_name() {
        let (service, account) =
            parse_secret_ref("ai.zyr1.vault:credential:test:api_key").expect("parse secret ref");

        assert_eq!(service, LEGACY_KEYCHAIN_SERVICE_NAME);
        assert_eq!(account, "credential:test:api_key");
    }

    #[test]
    fn parse_secret_ref_should_preserve_current_service_name() {
        let secret_ref = format!("{KEYCHAIN_SERVICE_NAME}:credential:test:api_key");
        let (service, account) = parse_secret_ref(&secret_ref).expect("parse secret ref");

        assert_eq!(service, KEYCHAIN_SERVICE_NAME);
        assert_eq!(account, "credential:test:api_key");
    }
}
