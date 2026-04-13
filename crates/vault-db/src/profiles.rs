use anyhow::Result;
use sqlx::Row;
use uuid::Uuid;
use vault_core::models::Profile;

use crate::{codec::parse_timestamp, store::Store};

impl Store {
    pub async fn insert_profile(&self, profile: &Profile) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO profiles (id, name, description, default_project, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
        )
        .bind(profile.id.to_string())
        .bind(&profile.name)
        .bind(&profile.description)
        .bind(&profile.default_project)
        .bind(profile.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_profiles(&self) -> Result<Vec<Profile>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, description, default_project, created_at
            FROM profiles
            ORDER BY name
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(map_profile_row).collect()
    }

    pub async fn get_profile_by_name(&self, name: &str) -> Result<Option<Profile>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, description, default_project, created_at
            FROM profiles
            WHERE name = ?1
            "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        row.map(map_profile_row).transpose()
    }

    pub async fn get_profile(&self, id: Uuid) -> Result<Option<Profile>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, description, default_project, created_at
            FROM profiles
            WHERE id = ?1
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        row.map(map_profile_row).transpose()
    }
}

fn map_profile_row(row: sqlx::sqlite::SqliteRow) -> Result<Profile> {
    Ok(Profile {
        id: Uuid::parse_str(row.get::<&str, _>("id"))?,
        name: row.get("name"),
        description: row.get("description"),
        default_project: row.get("default_project"),
        created_at: parse_timestamp(row.get::<&str, _>("created_at"))?,
    })
}
