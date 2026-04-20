use anyhow::Context;
use vault_db::Store;
use vault_secrets::{LEGACY_KEYCHAIN_SERVICE_NAME, migrate_legacy_secret_ref, parse_secret_ref};

pub async fn migrate_legacy_credentials_in_store(store: &Store) -> anyhow::Result<usize> {
    #[cfg(target_os = "macos")]
    {
        let current_exe =
            std::env::current_exe().context("failed to resolve current vault executable")?;
        let env_override = std::env::var("VAULT_TRUSTED_APP_PATHS").ok();
        let mut migrated = 0usize;

        for credential in store.list_credentials().await? {
            let (service, _) = parse_secret_ref(&credential.secret_ref)?;
            if service != LEGACY_KEYCHAIN_SERVICE_NAME {
                continue;
            }

            if let Some(new_ref) = migrate_legacy_secret_ref(
                &credential.secret_ref,
                &current_exe,
                env_override.as_deref(),
            )
            .await?
            {
                store
                    .update_credential_secret_ref(credential.id, &new_ref)
                    .await?;
                migrated += 1;
            }
        }

        Ok(migrated)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = store;
        Ok(0)
    }
}
