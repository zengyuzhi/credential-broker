use anyhow::Result;
use sqlx::Row;
use uuid::Uuid;
use vault_core::models::Capability;

use crate::{codec::parse_timestamp, store::Store};

impl Store {
    pub async fn insert_capability(&self, capability: &Capability) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO capabilities (id, connector_id, name, description, enabled, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )
        .bind(capability.id.to_string())
        .bind(capability.connector_id.to_string())
        .bind(&capability.name)
        .bind(&capability.description)
        .bind(i64::from(capability.enabled))
        .bind(capability.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_capabilities(&self) -> Result<Vec<Capability>> {
        let rows = sqlx::query(
            r#"
            SELECT id, connector_id, name, description, enabled, created_at
            FROM capabilities
            ORDER BY name
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_capability_row).collect()
    }

    pub async fn list_capabilities_for_connector(
        &self,
        connector_id: Uuid,
    ) -> Result<Vec<Capability>> {
        let rows = sqlx::query(
            r#"
            SELECT id, connector_id, name, description, enabled, created_at
            FROM capabilities WHERE connector_id = ?1
            ORDER BY name
            "#,
        )
        .bind(connector_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_capability_row).collect()
    }

    pub async fn get_capability(&self, id: Uuid) -> Result<Option<Capability>> {
        let row = sqlx::query(
            r#"
            SELECT id, connector_id, name, description, enabled, created_at
            FROM capabilities WHERE id = ?1
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.map(map_capability_row).transpose()
    }

    pub async fn get_capability_by_name(
        &self,
        connector_id: Uuid,
        name: &str,
    ) -> Result<Option<Capability>> {
        let row = sqlx::query(
            r#"
            SELECT id, connector_id, name, description, enabled, created_at
            FROM capabilities WHERE connector_id = ?1 AND name = ?2
            "#,
        )
        .bind(connector_id.to_string())
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        row.map(map_capability_row).transpose()
    }

    pub async fn delete_capability(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM capabilities WHERE id = ?1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

fn map_capability_row(row: sqlx::sqlite::SqliteRow) -> Result<Capability> {
    Ok(Capability {
        id: Uuid::parse_str(row.get::<&str, _>("id"))?,
        connector_id: Uuid::parse_str(row.get::<&str, _>("connector_id"))?,
        name: row.get("name"),
        description: row.get("description"),
        enabled: row.get::<i64, _>("enabled") != 0,
        created_at: parse_timestamp(row.get::<&str, _>("created_at"))?,
    })
}
