use anyhow::{Context, Result};
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;
use vault_core::models::UsageEvent;

use crate::{
    codec::{access_mode_as_str, parse_access_mode, parse_timestamp},
    store::Store,
};

#[derive(Debug, Clone, Serialize)]
pub struct ProviderStats {
    pub provider: String,
    pub request_count: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub estimated_cost_usd: f64,
    pub last_used_at: String,
}

impl Store {
    pub async fn insert_usage_event(&self, event: &UsageEvent) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO usage_events (
                id, provider, credential_id, lease_id, agent_name, project,
                mode, operation, endpoint, model, request_count,
                prompt_tokens, completion_tokens, total_tokens,
                estimated_cost_usd, status_code, success, latency_ms,
                error_text, created_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6,
                ?7, ?8, ?9, ?10, ?11,
                ?12, ?13, ?14,
                ?15, ?16, ?17, ?18,
                ?19, ?20
            )
            "#,
        )
        .bind(event.id.to_string())
        .bind(&event.provider)
        .bind(event.credential_id.to_string())
        .bind(event.lease_id.map(|id| id.to_string()))
        .bind(&event.agent_name)
        .bind(&event.project)
        .bind(access_mode_as_str(&event.mode))
        .bind(&event.operation)
        .bind(&event.endpoint)
        .bind(&event.model)
        .bind(event.request_count)
        .bind(event.prompt_tokens)
        .bind(event.completion_tokens)
        .bind(event.total_tokens)
        .bind(event.estimated_cost_usd)
        .bind(event.status_code)
        .bind(i64::from(event.success))
        .bind(event.latency_ms)
        .bind(&event.error_text)
        .bind(event.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn usage_stats_by_provider(&self) -> Result<Vec<ProviderStats>> {
        let rows = sqlx::query(
            "SELECT provider, \
             SUM(request_count) as request_count, \
             COALESCE(SUM(prompt_tokens), 0) as prompt_tokens, \
             COALESCE(SUM(completion_tokens), 0) as completion_tokens, \
             COALESCE(SUM(total_tokens), 0) as total_tokens, \
             COALESCE(SUM(estimated_cost_usd), 0.0) as estimated_cost_usd, \
             MAX(created_at) as last_used_at \
             FROM usage_events \
             GROUP BY provider \
             ORDER BY request_count DESC",
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to query usage stats by provider")?;

        rows.into_iter()
            .map(|row| {
                Ok(ProviderStats {
                    provider: row.get("provider"),
                    request_count: row.get("request_count"),
                    prompt_tokens: row.get("prompt_tokens"),
                    completion_tokens: row.get("completion_tokens"),
                    total_tokens: row.get("total_tokens"),
                    estimated_cost_usd: row.get("estimated_cost_usd"),
                    last_used_at: row.get("last_used_at"),
                })
            })
            .collect()
    }

    pub async fn list_usage_events(&self, limit: i64) -> Result<Vec<UsageEvent>> {
        let rows = sqlx::query(
            r#"
            SELECT id, provider, credential_id, lease_id, agent_name, project,
                   mode, operation, endpoint, model, request_count,
                   prompt_tokens, completion_tokens, total_tokens,
                   estimated_cost_usd, status_code, success, latency_ms,
                   error_text, created_at
            FROM usage_events
            ORDER BY created_at DESC
            LIMIT ?1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(map_usage_event_row).collect()
    }

    /// Stats for a single provider.
    pub async fn usage_stats_for_provider(&self, provider: &str) -> Result<Option<ProviderStats>> {
        let row = sqlx::query(
            "SELECT provider, \
             SUM(request_count) as request_count, \
             COALESCE(SUM(prompt_tokens), 0) as prompt_tokens, \
             COALESCE(SUM(completion_tokens), 0) as completion_tokens, \
             COALESCE(SUM(total_tokens), 0) as total_tokens, \
             COALESCE(SUM(estimated_cost_usd), 0.0) as estimated_cost_usd, \
             MAX(created_at) as last_used_at \
             FROM usage_events \
             WHERE provider = ?1 \
             GROUP BY provider",
        )
        .bind(provider)
        .fetch_optional(&self.pool)
        .await
        .context("failed to query usage stats for provider")?;

        row.map(|r| {
            Ok(ProviderStats {
                provider: r.get("provider"),
                request_count: r.get("request_count"),
                prompt_tokens: r.get("prompt_tokens"),
                completion_tokens: r.get("completion_tokens"),
                total_tokens: r.get("total_tokens"),
                estimated_cost_usd: r.get("estimated_cost_usd"),
                last_used_at: r.get("last_used_at"),
            })
        })
        .transpose()
    }

    /// Return the MAX(created_at) timestamp across all usage events, for SSE change detection.
    pub async fn max_usage_event_time(&self) -> Result<Option<String>> {
        let result: Option<String> =
            sqlx::query_scalar("SELECT MAX(created_at) FROM usage_events")
                .fetch_one(&self.pool)
                .await?;
        Ok(result)
    }

    /// List usage events filtered by provider.
    pub async fn list_usage_events_for_provider(
        &self,
        provider: &str,
        limit: i64,
    ) -> Result<Vec<UsageEvent>> {
        let rows = sqlx::query(
            "SELECT id, provider, credential_id, lease_id, agent_name, project, \
             mode, operation, endpoint, model, request_count, \
             prompt_tokens, completion_tokens, total_tokens, \
             estimated_cost_usd, status_code, success, latency_ms, \
             error_text, created_at \
             FROM usage_events \
             WHERE provider = ?1 \
             ORDER BY created_at DESC \
             LIMIT ?2",
        )
        .bind(provider)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_usage_event_row).collect()
    }
}

fn map_usage_event_row(row: sqlx::sqlite::SqliteRow) -> Result<UsageEvent> {
    let id = Uuid::parse_str(row.get::<&str, _>("id"))?;
    let credential_id = Uuid::parse_str(row.get::<&str, _>("credential_id"))?;
    let lease_id = row
        .get::<Option<String>, _>("lease_id")
        .map(|s| Uuid::parse_str(&s))
        .transpose()?;
    let mode = parse_access_mode(row.get::<&str, _>("mode"))?;
    let created_at = parse_timestamp(row.get::<&str, _>("created_at"))?;

    Ok(UsageEvent {
        id,
        provider: row.get("provider"),
        credential_id,
        lease_id,
        agent_name: row.get("agent_name"),
        project: row.get("project"),
        mode,
        operation: row.get("operation"),
        endpoint: row.get("endpoint"),
        model: row.get("model"),
        request_count: row.get("request_count"),
        prompt_tokens: row.get("prompt_tokens"),
        completion_tokens: row.get("completion_tokens"),
        total_tokens: row.get("total_tokens"),
        estimated_cost_usd: row.get("estimated_cost_usd"),
        status_code: row.get("status_code"),
        success: row.get::<i64, _>("success") != 0,
        latency_ms: row.get("latency_ms"),
        error_text: row.get("error_text"),
        created_at,
    })
}
