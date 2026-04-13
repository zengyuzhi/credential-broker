use chrono::Utc;
use tempfile::TempDir;
use uuid::Uuid;
use vault_core::models::{
    AccessMode, Credential, CredentialKind, Profile, ProfileBinding, UsageEvent,
};
use vault_db::{ProviderStats, Store};

fn sample_credential(provider: &str, label: &str) -> Credential {
    let now = Utc::now();
    Credential {
        id: Uuid::new_v4(),
        provider: provider.to_string(),
        kind: CredentialKind::ApiKey,
        label: label.to_string(),
        secret_ref: format!("ref:{provider}:{label}"),
        environment: "work".to_string(),
        owner: Some("zyz".to_string()),
        enabled: true,
        created_at: now,
        updated_at: now,
        last_used_at: None,
    }
}

fn sample_profile(name: &str) -> Profile {
    Profile {
        id: Uuid::new_v4(),
        name: name.to_string(),
        description: Some("test profile".to_string()),
        default_project: Some("sandbox".to_string()),
        created_at: Utc::now(),
    }
}

fn sample_binding(profile_id: Uuid, provider: &str, credential_id: Uuid) -> ProfileBinding {
    ProfileBinding {
        id: Uuid::new_v4(),
        profile_id,
        provider: provider.to_string(),
        credential_id,
        mode: AccessMode::Inject,
    }
}

struct TestStore {
    _dir: TempDir,
    store: Store,
}

async fn temp_store() -> TestStore {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_url = format!("sqlite:{}", dir.path().join("repo.db").display());
    let store = Store::connect(&db_url).await.expect("connect store");
    TestStore { _dir: dir, store }
}

#[tokio::test]
async fn insert_credential_should_be_listed() {
    let test_store = temp_store().await;
    let credential = sample_credential("openai", "work-main");

    test_store
        .store
        .insert_credential(&credential)
        .await
        .expect("insert credential");
    let credentials = test_store
        .store
        .list_credentials()
        .await
        .expect("list credentials");

    assert_eq!(credentials.len(), 1);
    assert_eq!(credentials[0].provider, "openai");
    assert_eq!(credentials[0].label, "work-main");
    assert!(matches!(credentials[0].kind, CredentialKind::ApiKey));
    assert_eq!(credentials[0].owner.as_deref(), Some("zyz"));
    assert!(credentials[0].enabled);
}

#[tokio::test]
async fn set_credential_enabled_should_update_enabled_flag() {
    let test_store = temp_store().await;
    let credential = sample_credential("openai", "toggle-me");

    test_store
        .store
        .insert_credential(&credential)
        .await
        .expect("insert credential");
    test_store
        .store
        .set_credential_enabled(credential.id, false)
        .await
        .expect("disable credential");

    let loaded = test_store
        .store
        .get_credential(credential.id)
        .await
        .expect("get credential")
        .expect("credential exists");

    assert!(!loaded.enabled);
}

#[tokio::test]
async fn delete_credential_should_remove_row() {
    let test_store = temp_store().await;
    let credential = sample_credential("coingecko", "market-data");

    test_store
        .store
        .insert_credential(&credential)
        .await
        .expect("insert credential");
    test_store
        .store
        .delete_credential(credential.id)
        .await
        .expect("delete credential");

    let loaded = test_store
        .store
        .get_credential(credential.id)
        .await
        .expect("get credential");

    assert!(loaded.is_none());
}

#[tokio::test]
async fn list_profiles_should_return_profiles_sorted_by_name() {
    let test_store = temp_store().await;
    let zed = sample_profile("zed");
    let alpha = sample_profile("alpha");

    test_store
        .store
        .insert_profile(&zed)
        .await
        .expect("insert zed profile");
    test_store
        .store
        .insert_profile(&alpha)
        .await
        .expect("insert alpha profile");

    let profiles = test_store
        .store
        .list_profiles()
        .await
        .expect("list profiles");

    assert_eq!(profiles.len(), 2);
    assert_eq!(profiles[0].name, "alpha");
    assert_eq!(profiles[1].name, "zed");
}

#[tokio::test]
async fn create_profile_and_binding_should_be_queryable() {
    let test_store = temp_store().await;
    let credential = sample_credential("twitterapi", "social-main");
    let profile = sample_profile("coding");
    let binding = sample_binding(profile.id, "twitterapi", credential.id);

    test_store
        .store
        .insert_credential(&credential)
        .await
        .expect("insert credential");
    test_store
        .store
        .insert_profile(&profile)
        .await
        .expect("insert profile");
    test_store
        .store
        .insert_binding(&binding)
        .await
        .expect("insert binding");

    let loaded_profile = test_store
        .store
        .get_profile_by_name("coding")
        .await
        .expect("query profile")
        .expect("profile exists");
    let bindings = test_store
        .store
        .list_bindings_for_profile(profile.id)
        .await
        .expect("list bindings");

    assert_eq!(loaded_profile.name, "coding");
    assert_eq!(loaded_profile.description.as_deref(), Some("test profile"));
    assert_eq!(loaded_profile.default_project.as_deref(), Some("sandbox"));
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].provider, "twitterapi");
    assert_eq!(bindings[0].credential_id, credential.id);
    assert!(matches!(bindings[0].mode, AccessMode::Inject));
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
async fn insert_usage_event_should_be_queryable() {
    let test_store = temp_store().await;
    let credential = sample_credential("openai", "work");

    test_store
        .store
        .insert_credential(&credential)
        .await
        .expect("insert credential");

    let event = sample_usage_event(credential.id);
    let event_id = event.id;

    test_store
        .store
        .insert_usage_event(&event)
        .await
        .expect("insert usage event");

    let events = test_store
        .store
        .list_usage_events(10)
        .await
        .expect("list usage events");

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id, event_id);
    assert_eq!(events[0].provider, "openai");
    assert_eq!(events[0].credential_id, credential.id);
    assert_eq!(events[0].agent_name, "test-agent");
    assert_eq!(events[0].project.as_deref(), Some("my-project"));
    assert!(matches!(events[0].mode, AccessMode::Inject));
    assert_eq!(events[0].operation, "chat_completion");
    assert_eq!(events[0].model.as_deref(), Some("gpt-4o"));
    assert_eq!(events[0].request_count, 1);
    assert_eq!(events[0].prompt_tokens, Some(100));
    assert_eq!(events[0].completion_tokens, Some(200));
    assert_eq!(events[0].total_tokens, Some(300));
    assert_eq!(events[0].status_code, Some(200));
    assert!(events[0].success);
    assert_eq!(events[0].latency_ms, 450);
    assert!(events[0].error_text.is_none());
}

#[tokio::test]
async fn usage_stats_by_provider_should_aggregate_events() {
    let test_store = temp_store().await;
    let credential = sample_credential("openai", "stats-test");

    test_store
        .store
        .insert_credential(&credential)
        .await
        .expect("insert credential");

    for _ in 0..3 {
        let event = UsageEvent {
            prompt_tokens: Some(100),
            completion_tokens: Some(50),
            total_tokens: Some(150),
            estimated_cost_usd: Some(0.01),
            ..sample_usage_event(credential.id)
        };
        test_store
            .store
            .insert_usage_event(&event)
            .await
            .expect("insert usage event");
    }

    let stats: Vec<ProviderStats> = test_store
        .store
        .usage_stats_by_provider()
        .await
        .expect("usage stats by provider");

    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].provider, "openai");
    assert_eq!(stats[0].request_count, 3);
    assert_eq!(stats[0].prompt_tokens, 300);
    assert_eq!(stats[0].completion_tokens, 150);
}

#[tokio::test]
async fn get_lease_by_token_hash_should_return_matching_lease() {
    let ts = temp_store().await;

    let profile = sample_profile("coding");
    ts.store.insert_profile(&profile).await.unwrap();

    let (lease, _raw_token) = vault_policy::lease::issue_lease(profile.id, "demo", None, 60);
    ts.store.insert_lease(&lease).await.unwrap();

    let found = ts
        .store
        .get_lease_by_token_hash(&lease.session_token_hash)
        .await
        .unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, lease.id);
}
