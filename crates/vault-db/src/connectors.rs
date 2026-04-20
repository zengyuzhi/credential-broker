use anyhow::Result;
use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;
use vault_core::models::Connector;

use crate::{codec::parse_timestamp, store::Store};

impl Store {
    pub async fn insert_connector(&self, connector: &Connector) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO connectors (id, name, provider, credential_id, base_url, enabled, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
        )
        .bind(connector.id.to_string())
        .bind(&connector.name)
        .bind(&connector.provider)
        .bind(connector.credential_id.to_string())
        .bind(&connector.base_url)
        .bind(i64::from(connector.enabled))
        .bind(connector.created_at.to_rfc3339())
        .bind(connector.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_connectors(&self) -> Result<Vec<Connector>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, provider, credential_id, base_url, enabled, created_at, updated_at
            FROM connectors
            ORDER BY name
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_connector_row).collect()
    }

    pub async fn get_connector(&self, id: Uuid) -> Result<Option<Connector>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, provider, credential_id, base_url, enabled, created_at, updated_at
            FROM connectors WHERE id = ?1
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.map(map_connector_row).transpose()
    }

    pub async fn get_connector_by_name(&self, name: &str) -> Result<Option<Connector>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, provider, credential_id, base_url, enabled, created_at, updated_at
            FROM connectors WHERE name = ?1
            "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        row.map(map_connector_row).transpose()
    }

    pub async fn set_connector_enabled(&self, id: Uuid, enabled: bool) -> Result<()> {
        sqlx::query("UPDATE connectors SET enabled = ?2, updated_at = ?3 WHERE id = ?1")
            .bind(id.to_string())
            .bind(i64::from(enabled))
            .bind(Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_connector(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM connectors WHERE id = ?1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

fn map_connector_row(row: sqlx::sqlite::SqliteRow) -> Result<Connector> {
    Ok(Connector {
        id: Uuid::parse_str(row.get::<&str, _>("id"))?,
        name: row.get("name"),
        provider: row.get("provider"),
        credential_id: Uuid::parse_str(row.get::<&str, _>("credential_id"))?,
        base_url: row.get("base_url"),
        enabled: row.get::<i64, _>("enabled") != 0,
        created_at: parse_timestamp(row.get::<&str, _>("created_at"))?,
        updated_at: parse_timestamp(row.get::<&str, _>("updated_at"))?,
    })
}
