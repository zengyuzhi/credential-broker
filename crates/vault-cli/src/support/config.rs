use std::{
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

fn db_override() -> &'static Mutex<Option<String>> {
    static DB_OVERRIDE: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    DB_OVERRIDE.get_or_init(|| Mutex::new(None))
}

fn default_database_url() -> String {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from)
        .expect("vault-cli should live under <workspace>/crates/vault-cli");
    format!(
        "sqlite:{}",
        workspace_root.join(".local/vault.db").display()
    )
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
    use super::default_database_url;

    #[test]
    fn default_database_url_should_point_to_workspace_local_db() {
        let database_url = default_database_url();
        assert!(database_url.starts_with("sqlite:/"));
        assert!(database_url.ends_with("/.local/vault.db"));
        assert!(database_url.contains("/credential-broker/"));
        assert!(!database_url.contains("crates/vault-cli/.local/vault.db"));
    }
}
