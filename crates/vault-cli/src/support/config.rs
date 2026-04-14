use std::{
    fs,
    path::Path,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

fn db_override() -> &'static Mutex<Option<String>> {
    static DB_OVERRIDE: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    DB_OVERRIDE.get_or_init(|| Mutex::new(None))
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from)
        .expect("vault-cli should live under <workspace>/crates/vault-cli")
}

fn workspace_local_dir() -> PathBuf {
    workspace_root().join(".local")
}

fn default_database_url() -> String {
    format!(
        "sqlite:{}",
        workspace_local_dir().join("vault.db").display()
    )
}

fn sqlite_path_from_url(database_url: &str) -> Option<PathBuf> {
    let raw_path = database_url.strip_prefix("sqlite:")?;
    let path_without_query = raw_path
        .split_once('?')
        .map(|(path, _)| path)
        .unwrap_or(raw_path);
    if path_without_query.is_empty() {
        return None;
    }
    Some(PathBuf::from(path_without_query))
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;

    let permissions = std::fs::Permissions::from_mode(mode);
    fs::set_permissions(path, permissions).expect("set path permissions");
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) {}

pub fn state_dir() -> PathBuf {
    let resolved = sqlite_path_from_url(&current_database_url())
        .and_then(|path| path.parent().map(PathBuf::from))
        .filter(|path| path.is_absolute())
        .unwrap_or_else(workspace_local_dir);

    fs::create_dir_all(&resolved).expect("create state directory");
    set_mode(&resolved, 0o700);
    resolved
}

pub fn current_database_url() -> String {
    if let Some(value) = db_override().lock().expect("db override lock").clone() {
        return value;
    }

    std::env::var("VAULT_DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(default_database_url)
}

#[cfg(test)]
pub fn set_test_database_url(database_url: impl Into<String>) {
    *db_override().lock().expect("db override lock") = Some(database_url.into());
}

#[cfg(test)]
pub fn clear_test_database_url() {
    *db_override().lock().expect("db override lock") = None;
}

#[cfg(test)]
pub fn test_database_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use tempfile::TempDir;

    use super::{
        clear_test_database_url, default_database_url, set_test_database_url, state_dir,
        test_database_lock,
    };

    #[test]
    fn default_database_url_should_point_to_workspace_local_db() {
        let database_url = default_database_url();
        assert!(database_url.starts_with("sqlite:/"));
        assert!(database_url.ends_with("/.local/vault.db"));
        assert!(database_url.contains("/credential-broker/"));
        assert!(!database_url.contains("crates/vault-cli/.local/vault.db"));
    }

    #[test]
    fn state_dir_should_follow_resolved_database_url_parent() {
        let _guard = test_database_lock().lock().expect("test lock");
        let dir = TempDir::new().expect("tempdir");
        let expected = dir.path().join("nested-state");
        let database_url = format!("sqlite:{}?mode=rwc", expected.join("vault.db").display());
        set_test_database_url(database_url);

        let resolved = state_dir();

        assert_eq!(resolved, expected);

        clear_test_database_url();
    }

    #[test]
    fn state_dir_should_create_directory_with_owner_only_permissions() {
        let _guard = test_database_lock().lock().expect("test lock");
        let dir = TempDir::new().expect("tempdir");
        let expected = dir.path().join("fresh-state");
        let database_url = format!("sqlite:{}?mode=rwc", expected.join("vault.db").display());
        set_test_database_url(database_url);

        let resolved = state_dir();
        let metadata = std::fs::metadata(&resolved).expect("state dir metadata");

        assert_eq!(resolved, expected);
        assert!(metadata.is_dir());
        assert_eq!(metadata.permissions().mode() & 0o777, 0o700);

        clear_test_database_url();
    }
}
