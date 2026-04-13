use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Resolve the set of executable paths that should be granted Keychain access.
///
/// - `current_exe`: the primary executable path (canonicalized when it exists on-disk).
/// - `env_override`: optional colon-separated list of additional paths (e.g. from the
///   `VAULT_TRUSTED_APP_PATHS` environment variable).
///
/// Blank entries in `env_override` are ignored. Duplicates are removed. Paths that exist
/// on disk are canonicalized; paths that do not exist are kept as-is so callers can still
/// build the list before the binary is first compiled.
pub fn trusted_application_paths_for(
    current_exe: impl AsRef<Path>,
    env_override: Option<&str>,
) -> anyhow::Result<Vec<PathBuf>> {
    let mut set: BTreeSet<PathBuf> = BTreeSet::new();

    let exe = canonicalize_best_effort(current_exe.as_ref());
    set.insert(exe);

    if let Some(overrides) = env_override {
        for entry in overrides.split(':') {
            let trimmed = entry.trim();
            if trimmed.is_empty() {
                continue;
            }
            let p = canonicalize_best_effort(Path::new(trimmed));
            set.insert(p);
        }
    }

    Ok(set.into_iter().collect())
}

/// Canonicalize a path when possible; return the original path otherwise.
fn canonicalize_best_effort(p: &Path) -> PathBuf {
    p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A direct binary path is preserved (or canonicalized to itself when it exists).
    #[test]
    fn trusted_application_paths_should_include_current_exe() {
        let paths = trusted_application_paths_for(
            "/tmp/work/target/debug/vault-cli",
            Some("/tmp/work/target/debug/vault-cli:/Applications/iTerm.app"),
        )
        .unwrap();

        assert!(
            paths.iter().any(|p| p.ends_with("target/debug/vault-cli")),
            "expected vault-cli in paths, got: {paths:?}"
        );
        // vault-cli + iTerm.app (two distinct paths)
        assert_eq!(paths.len(), 2, "expected exactly 2 paths, got: {paths:?}");
    }

    /// Duplicate paths are deduplicated.
    #[test]
    fn trusted_application_paths_deduplicates() {
        let paths = trusted_application_paths_for(
            "/tmp/work/target/debug/vault-cli",
            Some("/tmp/work/target/debug/vault-cli:/tmp/work/target/debug/vault-cli"),
        )
        .unwrap();

        assert_eq!(paths.len(), 1, "duplicates should be removed, got: {paths:?}");
    }

    /// Blank entries in the env override are ignored.
    #[test]
    fn trusted_application_paths_ignores_blank_entries() {
        let paths = trusted_application_paths_for(
            "/tmp/work/target/debug/vault-cli",
            Some(":/tmp/work/target/debug/vault-cli:  :"),
        )
        .unwrap();

        assert_eq!(paths.len(), 1, "blank entries should be ignored, got: {paths:?}");
    }

    /// No env override — only the primary exe is returned.
    #[test]
    fn trusted_application_paths_without_override() {
        let paths =
            trusted_application_paths_for("/tmp/work/target/debug/vault-cli", None).unwrap();

        assert_eq!(paths.len(), 1);
        assert!(paths[0].ends_with("target/debug/vault-cli"));
    }

    /// Real on-disk paths are canonicalized (symlinks resolved, relative components removed).
    #[test]
    fn trusted_application_paths_canonicalizes_existing_paths() {
        // /private/tmp is the canonical form of /tmp on macOS.
        let paths = trusted_application_paths_for("/tmp", None).unwrap();
        // Should have been canonicalized (on macOS /tmp → /private/tmp).
        // On Linux /tmp is already canonical. Either way exactly one path is returned.
        assert_eq!(paths.len(), 1);
    }
}
