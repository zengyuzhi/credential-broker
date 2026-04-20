use anyhow::Result;
use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;
use vault_core::models::Credential;

use crate::{
    codec::{credential_kind_as_str, parse_credential_kind, parse_timestamp},
    store::Store,
};

impl Store {
    pub async fn insert_credential(&self, credential: &Credential) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO credentials (
                id, provider, kind, label, secret_ref, environment, owner, enabled,
                created_at, updated_at, last_used_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
        )
        .bind(credential.id.to_string())
        .bind(&credential.provider)
        .bind(credential_kind_as_str(&credential.kind))
        .bind(&credential.label)
        .bind(&credential.secret_ref)
        .bind(&credential.environment)
        .bind(&credential.owner)
        .bind(i64::from(credential.enabled))
        .bind(credential.created_at.to_rfc3339())
        .bind(credential.updated_at.to_rfc3339())
        .bind(credential.last_used_at.map(|value| value.to_rfc3339()))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_credentials(&self) -> Result<Vec<Credential>> {
        let rows = sqlx::query(
            r#"
            SELECT id, provider, kind, label, secret_ref, environment, owner, enabled,
                   created_at, updated_at, last_used_at
            FROM credentials
            ORDER BY provider, label
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(map_credential_row).collect()
    }

    pub async fn get_credential(&self, id: Uuid) -> Result<Option<Credential>> {
        let row = sqlx::query(
            r#"
            SELECT id, provider, kind, label, secret_ref, environment, owner, enabled,
                   created_at, updated_at, last_used_at
            FROM credentials
            WHERE id = ?1
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        row.map(map_credential_row).transpose()
    }

    pub async fn set_credential_enabled(&self, id: Uuid, enabled: bool) -> Result<()> {
        sqlx::query("UPDATE credentials SET enabled = ?2, updated_at = ?3 WHERE id = ?1")
            .bind(id.to_string())
            .bind(i64::from(enabled))
            .bind(Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_credential_secret_ref(&self, id: Uuid, secret_ref: &str) -> Result<()> {
        sqlx::query("UPDATE credentials SET secret_ref = ?2, updated_at = ?3 WHERE id = ?1")
            .bind(id.to_string())
            .bind(secret_ref)
            .bind(Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_credential(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM credentials WHERE id = ?1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // Monotonic marker used by the dashboard SSE loop to detect cross-process
    // credential mutations (enable / disable / rename) that do not change the
    // row count — the blind spot UAT-FIND-005 exposed.
    pub async fn max_credential_updated_at(&self) -> Result<Option<String>> {
        let result: Option<String> = sqlx::query_scalar("SELECT MAX(updated_at) FROM credentials")
            .fetch_one(&self.pool)
            .await?;
        Ok(result)
    }
}

fn map_credential_row(row: sqlx::sqlite::SqliteRow) -> Result<Credential> {
    let id = Uuid::parse_str(row.get::<&str, _>("id"))?;
    let kind = parse_credential_kind(row.get::<&str, _>("kind"))?;
    let created_at = parse_timestamp(row.get::<&str, _>("created_at"))?;
    let updated_at = parse_timestamp(row.get::<&str, _>("updated_at"))?;
    let last_used_at = row
        .get::<Option<String>, _>("last_used_at")
        .map(|value| parse_timestamp(&value))
        .transpose()?;

    Ok(Credential {
        id,
        provider: row.get("provider"),
        kind,
        label: row.get("label"),
        secret_ref: row.get("secret_ref"),
        environment: row.get("environment"),
        owner: row.get("owner"),
        enabled: row.get::<i64, _>("enabled") != 0,
        created_at,
        updated_at,
        last_used_at,
    })
}
