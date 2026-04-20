use anyhow::Result;
use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;
use vault_core::models::{Bundle, Grant};

use crate::{codec::parse_timestamp, grants::map_grant_row, store::Store};

impl Store {
    pub async fn insert_bundle(&self, bundle: &Bundle) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO bundles (id, name, description, source_profile_id, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )
        .bind(bundle.id.to_string())
        .bind(&bundle.name)
        .bind(&bundle.description)
        .bind(bundle.source_profile_id.map(|v| v.to_string()))
        .bind(bundle.created_at.to_rfc3339())
        .bind(bundle.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_bundles(&self) -> Result<Vec<Bundle>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, description, source_profile_id, created_at, updated_at
            FROM bundles
            ORDER BY name
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_bundle_row).collect()
    }

    pub async fn get_bundle(&self, id: Uuid) -> Result<Option<Bundle>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, description, source_profile_id, created_at, updated_at
            FROM bundles WHERE id = ?1
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.map(map_bundle_row).transpose()
    }

    pub async fn get_bundle_by_name(&self, name: &str) -> Result<Option<Bundle>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, description, source_profile_id, created_at, updated_at
            FROM bundles WHERE name = ?1
            "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        row.map(map_bundle_row).transpose()
    }

    pub async fn add_grant_to_bundle(&self, bundle_id: Uuid, grant_id: Uuid) -> Result<()> {
        sqlx::query("INSERT OR IGNORE INTO bundle_grants (bundle_id, grant_id) VALUES (?1, ?2)")
            .bind(bundle_id.to_string())
            .bind(grant_id.to_string())
            .execute(&self.pool)
            .await?;

        sqlx::query("UPDATE bundles SET updated_at = ?2 WHERE id = ?1")
            .bind(bundle_id.to_string())
            .bind(Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_grants_for_bundle(&self, bundle_id: Uuid) -> Result<Vec<Grant>> {
        let rows = sqlx::query(
            r#"
            SELECT g.id, g.agent_name, g.capability_id, g.ttl_minutes, g.max_requests,
                   g.require_confirmation, g.enabled, g.created_at, g.expires_at
            FROM grants g
            INNER JOIN bundle_grants bg ON bg.grant_id = g.id
            WHERE bg.bundle_id = ?1
            ORDER BY g.created_at DESC
            "#,
        )
        .bind(bundle_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_grant_row).collect()
    }

    pub async fn delete_bundle(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM bundles WHERE id = ?1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

fn map_bundle_row(row: sqlx::sqlite::SqliteRow) -> Result<Bundle> {
    let source_profile_id = row
        .get::<Option<String>, _>("source_profile_id")
        .map(|v| Uuid::parse_str(&v))
        .transpose()?;

    Ok(Bundle {
        id: Uuid::parse_str(row.get::<&str, _>("id"))?,
        name: row.get("name"),
        description: row.get("description"),
        source_profile_id,
        created_at: parse_timestamp(row.get::<&str, _>("created_at"))?,
        updated_at: parse_timestamp(row.get::<&str, _>("updated_at"))?,
    })
}
