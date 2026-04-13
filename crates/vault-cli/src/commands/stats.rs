use anyhow::Result;
use clap::Args;
use vault_db::Store;

use crate::support::config::current_database_url;

#[derive(Debug, Args)]
pub struct StatsCommand {
    #[arg(long)]
    pub provider: Option<String>,
}

pub async fn run_stats_command(cmd: StatsCommand) -> Result<()> {
    let store = Store::connect(&current_database_url()).await?;
    let stats = store.usage_stats_by_provider().await?;

    if stats.is_empty() {
        println!("No usage events recorded yet.");
        return Ok(());
    }

    for stat in &stats {
        if let Some(ref filter) = cmd.provider
            && stat.provider != *filter
        {
            continue;
        }
        println!(
            "provider={} requests={} prompt_tokens={} completion_tokens={} cost_usd={:.4} last_used={}",
            stat.provider,
            stat.request_count,
            stat.prompt_tokens,
            stat.completion_tokens,
            stat.estimated_cost_usd,
            stat.last_used_at,
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use tempfile::TempDir;
    use uuid::Uuid;
    use vault_core::models::{AccessMode, Credential, CredentialKind, UsageEvent};
    use vault_db::Store;

    use super::{StatsCommand, run_stats_command};
    use crate::support::config::{
        clear_test_database_url, current_database_url, set_test_database_url, test_database_lock,
    };

    fn setup_test_db() -> TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_url = format!("sqlite:{}", dir.path().join("stats.db").display());
        set_test_database_url(db_url);
        dir
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn stats_should_show_empty_when_no_events() {
        let _guard = test_database_lock().lock().expect("test lock");
        let _dir = setup_test_db();

        let cmd = StatsCommand { provider: None };
        run_stats_command(cmd).await.expect("stats should succeed");

        clear_test_database_url();
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn stats_should_run_with_events() {
        let _guard = test_database_lock().lock().expect("test lock");
        let _dir = setup_test_db();

        let store = Store::connect(&current_database_url())
            .await
            .expect("connect store");

        // Insert a credential that the usage event references.
        let credential = Credential {
            id: Uuid::new_v4(),
            provider: "openai".to_string(),
            kind: CredentialKind::ApiKey,
            label: "test-key".to_string(),
            secret_ref: "ai.zyr1.vault:credential:test:api_key".to_string(),
            environment: "test".to_string(),
            owner: None,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_used_at: None,
        };
        store
            .insert_credential(&credential)
            .await
            .expect("insert credential");

        // Insert a usage event for the credential.
        let event = UsageEvent {
            id: Uuid::new_v4(),
            provider: "openai".to_string(),
            credential_id: credential.id,
            lease_id: None,
            agent_name: "test-agent".to_string(),
            project: None,
            mode: AccessMode::Inject,
            operation: "chat.completion".to_string(),
            endpoint: None,
            model: Some("gpt-4o".to_string()),
            request_count: 3,
            prompt_tokens: Some(100),
            completion_tokens: Some(50),
            total_tokens: Some(150),
            estimated_cost_usd: Some(0.0012),
            status_code: Some(200),
            success: true,
            latency_ms: 420,
            error_text: None,
            created_at: Utc::now(),
        };
        store
            .insert_usage_event(&event)
            .await
            .expect("insert usage event");

        // Run stats with no filter — should print the openai row.
        let cmd = StatsCommand { provider: None };
        run_stats_command(cmd).await.expect("stats should succeed");

        // Run stats with a provider filter that matches.
        let cmd = StatsCommand {
            provider: Some("openai".to_string()),
        };
        run_stats_command(cmd)
            .await
            .expect("stats with filter should succeed");

        // Run stats with a provider filter that does NOT match — should produce no output.
        let cmd = StatsCommand {
            provider: Some("anthropic".to_string()),
        };
        run_stats_command(cmd)
            .await
            .expect("stats with non-matching filter should succeed");

        clear_test_database_url();
    }
}
