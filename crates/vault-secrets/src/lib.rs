use async_trait::async_trait;

mod access;
pub use access::trusted_application_paths_for;

pub const KEYCHAIN_SERVICE_NAME: &str = "ai.zyr1.vault";

#[async_trait]
pub trait SecretStore: Send + Sync {
    async fn put(&self, service: &str, account: &str, secret: &str) -> anyhow::Result<String>;
    async fn get(&self, service: &str, account: &str) -> anyhow::Result<String>;
    async fn delete(&self, service: &str, account: &str) -> anyhow::Result<()>;
}

#[cfg(target_os = "macos")]
pub mod keychain;

#[cfg(target_os = "macos")]
pub use keychain::MacOsKeychainStore;
