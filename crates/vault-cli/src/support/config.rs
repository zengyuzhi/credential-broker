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

fn installed_state_dir(home_dir: &Path) -> PathBuf {
    home_dir
        .join(".local")
        .join("share")
        .join("credential-broker")
}

fn default_state_dir_from(workspace_root: &Path, home_dir: Option<&Path>) -> PathBuf {
    if workspace_root.exists() {
        return workspace_root.join(".local");
    }

    home_dir
        .map(installed_state_dir)
        .unwrap_or_else(|| workspace_root.join(".local"))
}

fn default_state_dir() -> PathBuf {
    let workspace = workspace_root();
    let home = std::env::var_os("HOME").map(PathBuf::from);
    default_state_dir_from(&workspace, home.as_deref())
}

fn default_database_url() -> String {
    format!("sqlite:{}", default_state_dir().join("vault.db").display())
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

pub fn resolved_state_dir() -> PathBuf {
    sqlite_path_from_url(&current_database_url())
        .and_then(|path| path.parent().map(PathBuf::from))
        .filter(|path| path.is_absolute())
        .unwrap_or_else(default_state_dir)
}

pub fn state_dir() -> PathBuf {
    let resolved = resolved_state_dir();

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
        clear_test_database_url, default_database_url, default_state_dir_from,
        set_test_database_url, state_dir, test_database_lock,
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
    fn default_state_dir_should_use_home_owned_path_when_workspace_is_missing() {
        let home = TempDir::new().expect("tempdir");
        let missing_workspace = home.path().join("missing-workspace");

        let resolved = default_state_dir_from(&missing_workspace, Some(home.path()));

        assert_eq!(
            resolved,
            home.path()
                .join(".local")
                .join("share")
                .join("credential-broker")
        );
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
