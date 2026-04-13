use anyhow::{Context, Result};
use sqlx::Row;
use uuid::Uuid;
use vault_core::models::Lease;

use crate::{codec::parse_timestamp, store::Store};

impl Store {
    pub async fn insert_lease(&self, lease: &Lease) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO leases (id, profile_id, agent_name, project, issued_at, expires_at, session_token_hash)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
        )
        .bind(lease.id.to_string())
        .bind(lease.profile_id.to_string())
        .bind(&lease.agent_name)
        .bind(&lease.project)
        .bind(lease.issued_at.to_rfc3339())
        .bind(lease.expires_at.to_rfc3339())
        .bind(&lease.session_token_hash)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_lease(&self, id: Uuid) -> Result<Option<Lease>> {
        let row = sqlx::query(
            r#"
            SELECT id, profile_id, agent_name, project, issued_at, expires_at, session_token_hash
            FROM leases
            WHERE id = ?1
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        row.map(map_lease_row).transpose()
    }

    pub async fn get_lease_by_token_hash(&self, token_hash: &str) -> Result<Option<Lease>> {
        let row = sqlx::query(
            "SELECT id, profile_id, agent_name, project, issued_at, expires_at, session_token_hash \
             FROM leases WHERE session_token_hash = ?",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
        .context("failed to look up lease by token hash")?;

        row.map(map_lease_row).transpose()
    }
}

fn map_lease_row(row: sqlx::sqlite::SqliteRow) -> Result<Lease> {
    Ok(Lease {
        id: Uuid::parse_str(row.get::<&str, _>("id"))?,
        profile_id: Uuid::parse_str(row.get::<&str, _>("profile_id"))?,
        agent_name: row.get("agent_name"),
        project: row.get("project"),
        issued_at: parse_timestamp(row.get::<&str, _>("issued_at"))?,
        expires_at: parse_timestamp(row.get::<&str, _>("expires_at"))?,
        session_token_hash: row.get("session_token_hash"),
    })
}
