use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Resolve the set of executable paths that should be granted Keychain access.
///
/// - `current_exe`: the primary executable path (canonicalized when it exists on-disk).
/// - `env_override`: optional colon-separated list of additional paths (e.g. from the
///   `VAULT_TRUSTED_APP_PATHS` environment variable). Colon-separated because this is
///   macOS-only (matching PATH conventions).
///
/// Blank entries in `env_override` are ignored. Duplicates are removed (both raw and
/// canonicalized forms are stored so dedup works regardless of whether a path exists
/// on disk at call time). Paths that exist on disk are canonicalized; paths that do not
/// exist are kept as-is so callers can still build the list before the binary is first
/// compiled.
pub fn trusted_application_paths_for(
    current_exe: impl AsRef<Path>,
    env_override: Option<&str>,
) -> Vec<PathBuf> {
    // Track both raw and canonical forms so the same logical path is never added twice,
    // regardless of whether it exists on disk at call time.
    let mut seen_raw: BTreeSet<PathBuf> = BTreeSet::new();
    let mut seen_canonical: BTreeSet<PathBuf> = BTreeSet::new();
    let mut result: Vec<PathBuf> = Vec::new();

    let mut insert = |raw: &Path| {
        let canonical = canonicalize_best_effort(raw);
        if seen_raw.contains(&raw.to_path_buf()) || seen_canonical.contains(&canonical) {
            return;
        }
        seen_raw.insert(raw.to_path_buf());
        seen_canonical.insert(canonical.clone());
        result.push(canonical);
    };

    insert(current_exe.as_ref());

    if let Some(overrides) = env_override {
        for entry in overrides.split(':') {
            let trimmed = entry.trim();
            if trimmed.is_empty() {
                continue;
            }
            insert(Path::new(trimmed));
        }
    }

    result
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
        // Use non-existent paths to avoid filesystem-dependent behavior.
        let paths = trusted_application_paths_for(
            "/nonexistent/target/debug/vault-cli",
            Some("/nonexistent/target/debug/vault-cli:/nonexistent/other/bin/tool"),
        );

        assert!(
            paths.iter().any(|p| p.ends_with("target/debug/vault-cli")),
            "expected vault-cli in paths, got: {paths:?}"
        );
        // vault-cli (deduped from override) + tool = 2 distinct paths
        assert_eq!(paths.len(), 2, "expected exactly 2 paths, got: {paths:?}");
    }

    /// Duplicate paths are deduplicated.
    #[test]
    fn trusted_application_paths_deduplicates() {
        let paths = trusted_application_paths_for(
            "/nonexistent/target/debug/vault-cli",
            Some("/nonexistent/target/debug/vault-cli:/nonexistent/target/debug/vault-cli"),
        );

        assert_eq!(paths.len(), 1, "duplicates should be removed, got: {paths:?}");
    }

    /// Blank entries in the env override are ignored.
    #[test]
    fn trusted_application_paths_ignores_blank_entries() {
        let paths = trusted_application_paths_for(
            "/nonexistent/target/debug/vault-cli",
            Some(":/nonexistent/target/debug/vault-cli:  :"),
        );

        assert_eq!(paths.len(), 1, "blank entries should be ignored, got: {paths:?}");
    }

    /// No env override — only the primary exe is returned.
    #[test]
    fn trusted_application_paths_without_override() {
        let paths =
            trusted_application_paths_for("/nonexistent/target/debug/vault-cli", None);

        assert_eq!(paths.len(), 1);
        assert!(paths[0].ends_with("target/debug/vault-cli"));
    }

    /// Real on-disk paths are canonicalized (symlinks resolved, relative components removed).
    #[test]
    fn trusted_application_paths_canonicalizes_existing_paths() {
        // /private/tmp is the canonical form of /tmp on macOS.
        let paths = trusted_application_paths_for("/tmp", None);
        assert_eq!(paths.len(), 1);
        // On macOS /tmp → /private/tmp; on Linux /tmp is already canonical.
        let resolved = &paths[0];
        assert!(
            resolved.to_str().unwrap().contains("tmp"),
            "expected a tmp-related path, got: {resolved:?}"
        );
    }

    /// Same logical path provided as raw and canonical form is deduplicated.
    #[test]
    fn trusted_application_paths_deduplicates_across_canonical_forms() {
        // /tmp and /private/tmp are the same path on macOS.
        let paths = trusted_application_paths_for("/tmp", Some("/private/tmp"));
        assert_eq!(paths.len(), 1, "canonical duplicates should be removed, got: {paths:?}");
    }
}
