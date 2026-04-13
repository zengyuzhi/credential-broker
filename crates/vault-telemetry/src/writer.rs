use vault_core::models::UsageEvent;
use vault_db::Store;

#[derive(Clone)]
pub struct TelemetryWriter {
    pub store: Store,
}

impl TelemetryWriter {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    pub async fn write_usage_event(&self, event: &UsageEvent) -> anyhow::Result<()> {
        self.store.insert_usage_event(event).await
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;
    use vault_core::models::{AccessMode, Credential, CredentialKind, UsageEvent};
    use vault_db::Store;

    use super::TelemetryWriter;

    async fn temp_store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_url = format!("sqlite:{}", dir.path().join("telemetry.db").display());
        let store = Store::connect(&db_url).await.expect("connect store");
        (dir, store)
    }

    fn sample_credential() -> Credential {
        let now = Utc::now();
        Credential {
            id: Uuid::new_v4(),
            provider: "openai".to_string(),
            kind: CredentialKind::ApiKey,
            label: "test-cred".to_string(),
            secret_ref: "ref:openai:test-cred".to_string(),
            environment: "work".to_string(),
            owner: None,
            enabled: true,
            created_at: now,
            updated_at: now,
            last_used_at: None,
        }
    }

    fn sample_usage_event(credential_id: Uuid) -> UsageEvent {
        UsageEvent {
            id: Uuid::new_v4(),
            provider: "openai".to_string(),
            credential_id,
            lease_id: None,
            agent_name: "test-agent".to_string(),
            project: Some("my-project".to_string()),
            mode: AccessMode::Inject,
            operation: "chat_completion".to_string(),
            endpoint: Some("/v1/chat/completions".to_string()),
            model: Some("gpt-4o".to_string()),
            request_count: 1,
            prompt_tokens: Some(100),
            completion_tokens: Some(200),
            total_tokens: Some(300),
            estimated_cost_usd: Some(0.005),
            status_code: Some(200),
            success: true,
            latency_ms: 450,
            error_text: None,
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn write_usage_event_should_persist_to_db() {
        let (_dir, store) = temp_store().await;
        let credential = sample_credential();
        store
            .insert_credential(&credential)
            .await
            .expect("insert credential");

        let writer = TelemetryWriter::new(store.clone());
        let event = sample_usage_event(credential.id);
        let event_id = event.id;

        writer
            .write_usage_event(&event)
            .await
            .expect("write usage event");

        let events = store
            .list_usage_events(10)
            .await
            .expect("list usage events");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, event_id);
        assert_eq!(events[0].provider, "openai");
        assert_eq!(events[0].agent_name, "test-agent");
        assert!(events[0].success);
    }
}
