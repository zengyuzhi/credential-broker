use anyhow::Result;
use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;
use vault_core::models::Session;

use crate::{codec::parse_timestamp, store::Store};

impl Store {
    pub async fn insert_session(&self, session: &Session) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO sessions (id, bundle_id, agent_name, project, issued_at, expires_at,
                                  session_token_hash, request_count)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
        )
        .bind(session.id.to_string())
        .bind(session.bundle_id.map(|v| v.to_string()))
        .bind(&session.agent_name)
        .bind(&session.project)
        .bind(session.issued_at.to_rfc3339())
        .bind(session.expires_at.to_rfc3339())
        .bind(&session.session_token_hash)
        .bind(session.request_count)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_session_grant(&self, session_id: Uuid, grant_id: Uuid) -> Result<()> {
        sqlx::query("INSERT OR IGNORE INTO session_grants (session_id, grant_id) VALUES (?1, ?2)")
            .bind(session_id.to_string())
            .bind(grant_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn insert_session_with_grants(
        &self,
        session: &Session,
        grant_ids: &[Uuid],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO sessions (id, bundle_id, agent_name, project, issued_at, expires_at,
                                  session_token_hash, request_count)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
        )
        .bind(session.id.to_string())
        .bind(session.bundle_id.map(|v| v.to_string()))
        .bind(&session.agent_name)
        .bind(&session.project)
        .bind(session.issued_at.to_rfc3339())
        .bind(session.expires_at.to_rfc3339())
        .bind(&session.session_token_hash)
        .bind(session.request_count)
        .execute(&mut *tx)
        .await?;

        for grant_id in grant_ids {
            sqlx::query(
                "INSERT OR IGNORE INTO session_grants (session_id, grant_id) VALUES (?1, ?2)",
            )
            .bind(session.id.to_string())
            .bind(grant_id.to_string())
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_session(&self, id: Uuid) -> Result<Option<Session>> {
        let row = sqlx::query(
            r#"
            SELECT id, bundle_id, agent_name, project, issued_at, expires_at,
                   session_token_hash, request_count
            FROM sessions WHERE id = ?1
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        row.map(map_session_row).transpose()
    }

    pub async fn get_broker_session_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<Session>> {
        let row = sqlx::query(
            r#"
            SELECT id, bundle_id, agent_name, project, issued_at, expires_at,
                   session_token_hash, request_count
            FROM sessions WHERE session_token_hash = ?1
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;
        row.map(map_session_row).transpose()
    }

    pub async fn list_active_sessions(&self) -> Result<Vec<Session>> {
        let now = Utc::now().to_rfc3339();
        let rows = sqlx::query(
            r#"
            SELECT id, bundle_id, agent_name, project, issued_at, expires_at,
                   session_token_hash, request_count
            FROM sessions WHERE expires_at > ?1
            ORDER BY issued_at DESC
            "#,
        )
        .bind(&now)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_session_row).collect()
    }

    pub async fn list_expired_sessions(&self, limit: i64) -> Result<Vec<Session>> {
        let now = Utc::now().to_rfc3339();
        let rows = sqlx::query(
            r#"
            SELECT id, bundle_id, agent_name, project, issued_at, expires_at,
                   session_token_hash, request_count
            FROM sessions WHERE expires_at <= ?1
            ORDER BY expires_at DESC LIMIT ?2
            "#,
        )
        .bind(&now)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_session_row).collect()
    }

    pub async fn increment_session_request_count(&self, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE sessions SET request_count = request_count + 1 WHERE id = ?1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_session(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM sessions WHERE id = ?1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

fn map_session_row(row: sqlx::sqlite::SqliteRow) -> Result<Session> {
    let bundle_id = row
        .get::<Option<String>, _>("bundle_id")
        .map(|v| Uuid::parse_str(&v))
        .transpose()?;

    Ok(Session {
        id: Uuid::parse_str(row.get::<&str, _>("id"))?,
        bundle_id,
        agent_name: row.get("agent_name"),
        project: row.get("project"),
        issued_at: parse_timestamp(row.get::<&str, _>("issued_at"))?,
        expires_at: parse_timestamp(row.get::<&str, _>("expires_at"))?,
        session_token_hash: row.get("session_token_hash"),
        request_count: row.get("request_count"),
    })
}
