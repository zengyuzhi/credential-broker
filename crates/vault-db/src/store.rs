use std::{
    borrow::Cow,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
    sync::LazyLock,
};

use anyhow::Context;
use sqlx::{
    SqlitePool,
    migrate::{Migration, MigrationType, Migrator},
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

static MIGRATOR: LazyLock<Migrator> = LazyLock::new(|| Migrator {
    migrations: Cow::Owned(vec![
        Migration::new(
            1,
            Cow::Borrowed("init"),
            MigrationType::Simple,
            Cow::Borrowed(include_str!("../../../migrations/0001_init.sql")),
            false,
        ),
        Migration::new(
            2,
            Cow::Borrowed("ui sessions"),
            MigrationType::Simple,
            Cow::Borrowed(include_str!("../../../migrations/0002_ui_sessions.sql")),
            false,
        ),
        Migration::new(
            3,
            Cow::Borrowed("usage events cost micros"),
            MigrationType::Simple,
            Cow::Borrowed(include_str!(
                "../../../migrations/0003_usage_events_cost_micros.sql"
            )),
            false,
        ),
    ]),
    ..Migrator::DEFAULT
});

#[derive(Clone)]
pub struct Store {
    pub pool: SqlitePool,
}

impl Store {
    pub async fn connect(database_url: &str) -> anyhow::Result<Self> {
        ensure_parent_directory(database_url)?;

        let options = SqliteConnectOptions::from_str(database_url)?
            .create_if_missing(true)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        MIGRATOR.run(&pool).await?;

        Ok(Self { pool })
    }
}

fn ensure_parent_directory(database_url: &str) -> anyhow::Result<()> {
    let Some(path) = sqlite_path_from_url(database_url) else {
        return Ok(());
    };

    if let Some(parent) = path.parent().filter(|value| !value.as_os_str().is_empty()) {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create parent directory {} for sqlite database {}",
                parent.display(),
                path.display()
            )
        })?;
    }

    Ok(())
}

fn sqlite_path_from_url(database_url: &str) -> Option<PathBuf> {
    let raw_path = database_url.strip_prefix("sqlite:")?;
    if raw_path.is_empty()
        || raw_path == ":memory:"
        || raw_path.starts_with("file::memory:")
        || raw_path.contains("mode=memory")
    {
        return None;
    }

    let path_without_query = raw_path.split('?').next().unwrap_or(raw_path);
    let trimmed = if let Some(rest) = path_without_query.strip_prefix("///") {
        format!("/{}", rest)
    } else if let Some(rest) = path_without_query.strip_prefix("//") {
        rest.to_string()
    } else {
        path_without_query.to_string()
    };

    if trimmed.is_empty() || trimmed == ":memory:" {
        return None;
    }

    Some(Path::new(&trimmed).to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::sqlite_path_from_url;

    #[test]
    fn sqlite_path_from_url_returns_relative_path_for_double_slash_form() {
        let path = sqlite_path_from_url("sqlite://.local/test.db?mode=rwc").expect("path");
        assert_eq!(path.to_string_lossy(), ".local/test.db");
    }

    #[test]
    fn sqlite_path_from_url_returns_none_for_in_memory_databases() {
        assert!(sqlite_path_from_url("sqlite::memory:").is_none());
        assert!(sqlite_path_from_url("sqlite:file::memory:?cache=shared").is_none());
    }
}
