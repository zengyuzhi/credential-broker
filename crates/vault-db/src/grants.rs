use anyhow::Result;
use sqlx::Row;
use uuid::Uuid;
use vault_core::models::Grant;

use crate::{codec::parse_timestamp, store::Store};

impl Store {
    pub async fn insert_grant(&self, grant: &Grant) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO grants (id, agent_name, capability_id, ttl_minutes, max_requests,
                                require_confirmation, enabled, created_at, expires_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
        )
        .bind(grant.id.to_string())
        .bind(&grant.agent_name)
        .bind(grant.capability_id.to_string())
        .bind(grant.ttl_minutes)
        .bind(grant.max_requests)
        .bind(i64::from(grant.require_confirmation))
        .bind(i64::from(grant.enabled))
        .bind(grant.created_at.to_rfc3339())
        .bind(grant.expires_at.map(|v| v.to_rfc3339()))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_grants(&self) -> Result<Vec<Grant>> {
        let rows = sqlx::query(
            r#"
            SELECT id, agent_name, capability_id, ttl_minutes, max_requests,
                   require_confirmation, enabled, created_at, expires_at
            FROM grants
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_grant_row).collect()
    }

    pub async fn list_grants_for_agent(&self, agent_name: &str) -> Result<Vec<Grant>> {
        let rows = sqlx::query(
            r#"
            SELECT id, agent_name, capability_id, ttl_minutes, max_requests,
                   require_confirmation, enabled, created_at, expires_at
            FROM grants WHERE agent_name = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(agent_name)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_grant_row).collect()
    }

    pub async fn list_grants_for_capability(&self, capability_id: Uuid) -> Result<Vec<Grant>> {
        let rows = sqlx::query(
            r#"
            SELECT id, agent_name, capability_id, ttl_minutes, max_requests,
                   require_confirmation, enabled, created_at, expires_at
            FROM grants WHERE capability_id = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(capability_id.to_string())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_grant_row).collect()
    }

    pub async fn get_grant(&self, id: Uuid) -> Result<Option<Grant>> {
        let row = sqlx::query(
            r#"
            SELECT id, agent_name, capability_id, ttl_minutes, max_requests,
                   require_confirmation, enabled, created_at, expires_at
            FROM grants WHERE id = ?1
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.map(map_grant_row).transpose()
    }

    pub async fn set_grant_enabled(&self, id: Uuid, enabled: bool) -> Result<()> {
        sqlx::query("UPDATE grants SET enabled = ?2 WHERE id = ?1")
            .bind(id.to_string())
            .bind(i64::from(enabled))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_grant(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM grants WHERE id = ?1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

pub(crate) fn map_grant_row(row: sqlx::sqlite::SqliteRow) -> Result<Grant> {
    let expires_at = row
        .get::<Option<String>, _>("expires_at")
        .map(|v| parse_timestamp(&v))
        .transpose()?;

    Ok(Grant {
        id: Uuid::parse_str(row.get::<&str, _>("id"))?,
        agent_name: row.get("agent_name"),
        capability_id: Uuid::parse_str(row.get::<&str, _>("capability_id"))?,
        ttl_minutes: row.get("ttl_minutes"),
        max_requests: row.get("max_requests"),
        require_confirmation: row.get::<i64, _>("require_confirmation") != 0,
        enabled: row.get::<i64, _>("enabled") != 0,
        created_at: parse_timestamp(row.get::<&str, _>("created_at"))?,
        expires_at,
    })
}
