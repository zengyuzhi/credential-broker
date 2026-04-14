use async_trait::async_trait;
use zeroize::Zeroizing;

mod access;
pub use access::trusted_application_paths_for;

pub const KEYCHAIN_SERVICE_NAME: &str = "ai.zyr1.vault";

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
