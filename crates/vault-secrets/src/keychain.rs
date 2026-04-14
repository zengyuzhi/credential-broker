use std::collections::BTreeSet;
use std::path::PathBuf;

use anyhow::Context;
use async_trait::async_trait;
use security_framework::os::macos::keychain::SecKeychain;
use security_framework::passwords::{delete_generic_password, get_generic_password};
use tokio::io::AsyncWriteExt;
use zeroize::Zeroizing;

use crate::SecretStore;

/// Options describing how a generic password item should be written to the Keychain.
///
/// Used as a pure-data value by `generic_password_options_with_trusted_apps` so that the
/// ACL-configuration logic can be tested without touching the live Keychain.
#[cfg(target_os = "macos")]
#[derive(Debug, Clone)]
pub struct PasswordOptions {
    /// Keychain service name.
    pub service: String,
    /// Keychain account name.
    pub account: String,
    /// Deduplicated list of application paths that should be granted silent access.
    pub trusted_apps: Vec<PathBuf>,
}

/// Build a `PasswordOptions` value describing a generic password item with explicit trusted-app
/// access control.
///
/// # Errors
///
/// Returns an error when `trusted_apps` is empty, because an empty ACL list is a
/// configuration mistake (it would leave the item inaccessible without a prompt to every
/// application).
#[cfg(target_os = "macos")]
pub fn generic_password_options_with_trusted_apps(
    service: &str,
    account: &str,
    trusted_apps: &[PathBuf],
) -> anyhow::Result<PasswordOptions> {
    anyhow::ensure!(
        !trusted_apps.is_empty(),
        "trusted_apps must not be empty: at least one application path is required"
    );

    // Deduplicate while preserving first-seen order.
    let mut seen: BTreeSet<PathBuf> = BTreeSet::new();
    let deduped: Vec<PathBuf> = trusted_apps
        .iter()
        .filter(|p| seen.insert((*p).clone()))
        .cloned()
        .collect();

    Ok(PasswordOptions {
        service: service.to_owned(),
        account: account.to_owned(),
        trusted_apps: deduped,
    })
}

#[derive(Debug, Default)]
pub struct MacOsKeychainStore;

impl MacOsKeychainStore {
    /// Store a generic password with explicit trusted-application ACL.
    ///
    /// Uses the `security` CLI with `-T appPath` flags so that the listed applications
    /// can read the item without triggering a user-interaction prompt. The item is
    /// created or updated (`-U` flag) atomically by the system tool.
    ///
    /// The secret is piped via stdin by placing `-w` as the final argument with no
    /// value, which causes `security` to read the password from stdin rather than
    /// accepting it as a CLI argument visible in process listings.
    ///
    /// The `secret_ref` returned follows the existing `"service:account"` convention.
    pub async fn put_with_access(
        &self,
        service: &str,
        account: &str,
        secret: &str,
        trusted_apps: &[PathBuf],
    ) -> anyhow::Result<String> {
        let opts = generic_password_options_with_trusted_apps(service, account, trusted_apps)?;

        // Use the absolute path to avoid PATH-based hijacking on the secret-ingest path.
        let mut cmd = tokio::process::Command::new("/usr/bin/security");
        cmd.arg("add-generic-password")
            .arg("-s")
            .arg(&opts.service)
            .arg("-a")
            .arg(&opts.account)
            .arg("-U"); // update if already exists

        for app_path in &opts.trusted_apps {
            cmd.arg("-T").arg(app_path);
        }

        // Place `-w` last with no value so `security` reads the password from stdin,
        // keeping the secret out of process argument lists.
        cmd.arg("-w");

        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd
            .spawn()
            .context("failed to spawn `/usr/bin/security add-generic-password`")?;

        // Write the secret to stdin then close it so `security` proceeds.
        // Copy the bytes into a `Zeroizing<Vec<u8>>` so the intermediate
        // buffer is wiped on drop rather than lingering in heap memory after
        // the pipe closes. Audit ZA-0007.
        if let Some(mut stdin) = child.stdin.take() {
            let buf: Zeroizing<Vec<u8>> = Zeroizing::new(secret.as_bytes().to_vec());
            stdin
                .write_all(&buf)
                .await
                .context("failed to write secret to security stdin")?;
            stdin.shutdown().await.ok();
        }

        let output = child
            .wait_with_output()
            .await
            .context("failed to wait for `/usr/bin/security add-generic-password`")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("security add-generic-password failed for {service}/{account}: {stderr}");
        }

        Ok(format!("{service}:{account}"))
    }
}

#[async_trait]
impl SecretStore for MacOsKeychainStore {
    async fn get(&self, service: &str, account: &str) -> anyhow::Result<Zeroizing<String>> {
        let _interaction_lock = SecKeychain::disable_user_interaction()
            .context("failed to disable macOS keychain user interaction")?;
        // `get_generic_password` returns `Vec<u8>` owned by us; wrap in
        // `Zeroizing` so the intermediate byte buffer is wiped on drop even
        // if `from_utf8` fails. Audit ZA-0001.
        let bytes = Zeroizing::new(
            get_generic_password(service, account)
                .with_context(|| format!("failed to load secret for {service}/{account}"))?,
        );
        let secret =
            String::from_utf8(bytes.to_vec()).context("keychain secret is not valid utf-8")?;
        Ok(Zeroizing::new(secret))
    }

    async fn delete(&self, service: &str, account: &str) -> anyhow::Result<()> {
        delete_generic_password(service, account)
            .with_context(|| format!("failed to delete secret for {service}/{account}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------------------
    // generic_password_options_with_trusted_apps — unit tests (no live Keychain)
    // ---------------------------------------------------------------------------

    /// Service and account names are preserved verbatim in the returned options.
    #[test]
    fn options_preserves_service_and_account() {
        let apps = vec![PathBuf::from("/usr/local/bin/vault-cli")];
        let opts =
            generic_password_options_with_trusted_apps("my-service", "my-account", &apps).unwrap();

        assert_eq!(opts.service, "my-service");
        assert_eq!(opts.account, "my-account");
    }

    /// An empty trusted-apps slice must produce an error.
    #[test]
    fn options_rejects_empty_trusted_apps() {
        let result = generic_password_options_with_trusted_apps("svc", "acct", &[]);
        assert!(
            result.is_err(),
            "expected an error for empty trusted_apps, got: {result:?}"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("trusted_apps must not be empty"),
            "unexpected error message: {msg}"
        );
    }

    /// Duplicate paths are removed; the first occurrence is kept.
    #[test]
    fn options_deduplicates_trusted_apps() {
        let path = PathBuf::from("/usr/local/bin/vault-cli");
        let apps = vec![path.clone(), path.clone(), path.clone()];
        let opts = generic_password_options_with_trusted_apps("svc", "acct", &apps).unwrap();

        assert_eq!(
            opts.trusted_apps.len(),
            1,
            "expected exactly 1 unique path after dedup, got: {:?}",
            opts.trusted_apps
        );
        assert_eq!(opts.trusted_apps[0], path);
    }

    /// Multiple distinct paths are all retained.
    #[test]
    fn options_retains_distinct_trusted_apps() {
        let a = PathBuf::from("/usr/local/bin/vault-cli");
        let b = PathBuf::from("/opt/homebrew/bin/vault-cli");
        let apps = vec![a.clone(), b.clone()];
        let opts = generic_password_options_with_trusted_apps("svc", "acct", &apps).unwrap();

        assert_eq!(opts.trusted_apps.len(), 2);
        assert!(opts.trusted_apps.contains(&a));
        assert!(opts.trusted_apps.contains(&b));
    }

    /// Mixed duplicates and distinct paths: only uniques survive.
    #[test]
    fn options_deduplicates_mixed_paths() {
        let a = PathBuf::from("/usr/local/bin/vault-cli");
        let b = PathBuf::from("/opt/homebrew/bin/vault-cli");
        let apps = vec![a.clone(), b.clone(), a.clone()];
        let opts = generic_password_options_with_trusted_apps("svc", "acct", &apps).unwrap();

        assert_eq!(
            opts.trusted_apps.len(),
            2,
            "expected 2 unique paths, got: {:?}",
            opts.trusted_apps
        );
    }

    /// `put_with_access` rejects empty trusted_apps before spawning any process.
    #[tokio::test]
    async fn put_with_access_rejects_empty_trusted_apps() {
        let store = MacOsKeychainStore;
        let result = store.put_with_access("svc", "acct", "secret", &[]).await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("trusted_apps must not be empty"),
            "unexpected error: {msg}"
        );
    }
}
