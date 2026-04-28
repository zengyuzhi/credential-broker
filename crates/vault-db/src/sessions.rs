use anyhow::Result;
use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;
use vault_core::models::{Capability, Connector, Grant, Session};

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

    /// Phase 1.1: increment the per-grant counter on `session_grants` so the
    /// proxy can enforce `Grant.max_requests`. Session-wide `request_count`
    /// remains a separate observability counter.
    pub async fn increment_session_grant_request_count(
        &self,
        session_id: Uuid,
        grant_id: Uuid,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE session_grants SET request_count = request_count + 1 \
             WHERE session_id = ?1 AND grant_id = ?2",
        )
        .bind(session_id.to_string())
        .bind(grant_id.to_string())
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

    /// Phase 1.1: list every enabled grant on this session that authorizes
    /// the given provider, alongside its capability, connector, and the
    /// per-session request counter used for `max_requests` enforcement.
    ///
    /// Caller-side disambiguation: when this returns multiple candidates the
    /// caller must pick one via connector/capability hints, otherwise reject
    /// with an ambiguity error. The SQL filter here matches only enabled
    /// entities; expiry and per-grant quota are enforced in the caller for
    /// easier error attribution.
    pub async fn list_session_proxy_candidates(
        &self,
        session_id: Uuid,
        provider: &str,
    ) -> Result<Vec<SessionGrantCandidate>> {
        let rows = sqlx::query(
            r#"
            SELECT
                g.id              AS grant_id,
                g.agent_name      AS agent_name,
                g.capability_id   AS capability_id,
                g.ttl_minutes     AS ttl_minutes,
                g.max_requests    AS max_requests,
                g.require_confirmation AS require_confirmation,
                g.enabled         AS enabled,
                g.created_at      AS created_at,
                g.expires_at      AS expires_at,
                c.id              AS cap_id,
                c.connector_id    AS cap_connector_id,
                c.name            AS cap_name,
                c.description     AS cap_description,
                c.enabled         AS cap_enabled,
                c.created_at      AS cap_created_at,
                cn.id             AS cn_id,
                cn.name           AS cn_name,
                cn.provider       AS cn_provider,
                cn.credential_id  AS cn_credential_id,
                cn.base_url       AS cn_base_url,
                cn.enabled        AS cn_enabled,
                cn.created_at     AS cn_created_at,
                cn.updated_at     AS cn_updated_at,
                sg.request_count  AS sg_request_count
            FROM session_grants sg
            INNER JOIN grants g       ON g.id = sg.grant_id
            INNER JOIN capabilities c ON c.id = g.capability_id
            INNER JOIN connectors cn  ON cn.id = c.connector_id
            WHERE sg.session_id = ?1
              AND cn.provider = ?2
              AND g.enabled = 1
              AND c.enabled = 1
              AND cn.enabled = 1
            ORDER BY g.created_at ASC
            "#,
        )
        .bind(session_id.to_string())
        .bind(provider)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(map_candidate_row).collect()
    }
}

/// Phase 1.1: one row returned by `list_session_proxy_candidates`. Bundles a
/// grant together with its capability, connector, and the per-session counter
/// needed for `Grant.max_requests` enforcement at the proxy boundary.
#[derive(Debug, Clone)]
pub struct SessionGrantCandidate {
    pub grant: Grant,
    pub capability: Capability,
    pub connector: Connector,
    pub session_grant_request_count: i64,
}

fn map_candidate_row(row: sqlx::sqlite::SqliteRow) -> Result<SessionGrantCandidate> {
    let grant = map_grant_row_with_prefix(&row)?;
    let capability = Capability {
        id: Uuid::parse_str(row.get::<&str, _>("cap_id"))?,
        connector_id: Uuid::parse_str(row.get::<&str, _>("cap_connector_id"))?,
        name: row.get("cap_name"),
        description: row.get("cap_description"),
        enabled: row.get::<i64, _>("cap_enabled") != 0,
        created_at: parse_timestamp(row.get::<&str, _>("cap_created_at"))?,
    };
    let connector = Connector {
        id: Uuid::parse_str(row.get::<&str, _>("cn_id"))?,
        name: row.get("cn_name"),
        provider: row.get("cn_provider"),
        credential_id: Uuid::parse_str(row.get::<&str, _>("cn_credential_id"))?,
        base_url: row.get("cn_base_url"),
        enabled: row.get::<i64, _>("cn_enabled") != 0,
        created_at: parse_timestamp(row.get::<&str, _>("cn_created_at"))?,
        updated_at: parse_timestamp(row.get::<&str, _>("cn_updated_at"))?,
    };

    Ok(SessionGrantCandidate {
        grant,
        capability,
        connector,
        session_grant_request_count: row.get("sg_request_count"),
    })
}

// Helper: the joined query above aliases grant columns with their natural
// names (id→grant_id, etc.). Reconstruct a Grant by rebinding those aliases
// to a fresh SqliteRow is cleaner than a second query, but sqlx does not let
// us rename columns on a borrowed row — so we build the Grant inline.
fn map_grant_row_with_prefix(row: &sqlx::sqlite::SqliteRow) -> Result<Grant> {
    let expires_at = row
        .get::<Option<String>, _>("expires_at")
        .map(|v| parse_timestamp(&v))
        .transpose()?;

    Ok(Grant {
        id: Uuid::parse_str(row.get::<&str, _>("grant_id"))?,
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
