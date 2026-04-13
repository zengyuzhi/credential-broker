use anyhow::Context;
use async_trait::async_trait;
use security_framework::os::macos::keychain::SecKeychain;
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

use crate::SecretStore;

#[derive(Debug, Default)]
pub struct MacOsKeychainStore;

#[async_trait]
impl SecretStore for MacOsKeychainStore {
    async fn put(&self, service: &str, account: &str, secret: &str) -> anyhow::Result<String> {
        set_generic_password(service, account, secret.as_bytes())
            .with_context(|| format!("failed to store secret for {service}/{account}"))?;
        Ok(format!("{service}:{account}"))
    }

    async fn get(&self, service: &str, account: &str) -> anyhow::Result<String> {
        let _interaction_lock = SecKeychain::disable_user_interaction()
            .context("failed to disable macOS keychain user interaction")?;
        let bytes = get_generic_password(service, account)
            .with_context(|| format!("failed to load secret for {service}/{account}"))?;
        let secret = String::from_utf8(bytes).context("keychain secret is not valid utf-8")?;
        Ok(secret)
    }

    async fn delete(&self, service: &str, account: &str) -> anyhow::Result<()> {
        delete_generic_password(service, account)
            .with_context(|| format!("failed to delete secret for {service}/{account}"))?;
        Ok(())
    }
}
