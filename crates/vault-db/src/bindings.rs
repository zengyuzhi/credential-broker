use anyhow::Result;
use sqlx::Row;
use uuid::Uuid;
use vault_core::models::ProfileBinding;

use crate::{
    codec::{access_mode_as_str, parse_access_mode},
    store::Store,
};

impl Store {
    pub async fn insert_binding(&self, binding: &ProfileBinding) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO profile_bindings (id, profile_id, provider, credential_id, mode)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(profile_id, provider)
            DO UPDATE SET credential_id = excluded.credential_id, mode = excluded.mode
            "#,
        )
        .bind(binding.id.to_string())
        .bind(binding.profile_id.to_string())
        .bind(&binding.provider)
        .bind(binding.credential_id.to_string())
        .bind(access_mode_as_str(&binding.mode))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_bindings_for_profile(&self, profile_id: Uuid) -> Result<Vec<ProfileBinding>> {
        let rows = sqlx::query(
            r#"
            SELECT id, profile_id, provider, credential_id, mode
            FROM profile_bindings
            WHERE profile_id = ?1
            ORDER BY provider
            "#,
        )
        .bind(profile_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(map_binding_row).collect()
    }
}

fn map_binding_row(row: sqlx::sqlite::SqliteRow) -> Result<ProfileBinding> {
    Ok(ProfileBinding {
        id: Uuid::parse_str(row.get::<&str, _>("id"))?,
        profile_id: Uuid::parse_str(row.get::<&str, _>("profile_id"))?,
        provider: row.get("provider"),
        credential_id: Uuid::parse_str(row.get::<&str, _>("credential_id"))?,
        mode: parse_access_mode(row.get::<&str, _>("mode"))?,
    })
}
