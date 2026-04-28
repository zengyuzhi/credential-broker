use chrono::Utc;
use uuid::Uuid;
use vault_core::models::{
    Bundle, Capability, Connector, Credential, CredentialKind, Grant, Session,
};
use vault_db::Store;

async fn test_store() -> Store {
    Store::connect("sqlite::memory:")
        .await
        .expect("in-memory store")
}

fn test_credential() -> Credential {
    let now = Utc::now();
    Credential {
        id: Uuid::new_v4(),
        provider: "openai".into(),
        kind: CredentialKind::ApiKey,
        label: "test-key".into(),
        secret_ref: "dev.credential-broker.vault:test:api_key".into(),
        environment: "work".into(),
        owner: None,
        enabled: true,
        created_at: now,
        updated_at: now,
        last_used_at: None,
    }
}

#[tokio::test]
async fn connector_crud() {
    let store = test_store().await;
    let cred = test_credential();
    store.insert_credential(&cred).await.expect("insert cred");

    let now = Utc::now();
    let connector = Connector {
        id: Uuid::new_v4(),
        name: "my-openai".into(),
        provider: "openai".into(),
        credential_id: cred.id,
        base_url: None,
        enabled: true,
        created_at: now,
        updated_at: now,
    };
    store.insert_connector(&connector).await.expect("insert");

    let fetched = store
        .get_connector_by_name("my-openai")
        .await
        .expect("get")
        .expect("found");
    assert_eq!(fetched.id, connector.id);
    assert_eq!(fetched.provider, "openai");

    let list = store.list_connectors().await.expect("list");
    assert_eq!(list.len(), 1);

    store
        .set_connector_enabled(connector.id, false)
        .await
        .expect("disable");
    let disabled = store
        .get_connector(connector.id)
        .await
        .expect("get")
        .expect("found");
    assert!(!disabled.enabled);

    store.delete_connector(connector.id).await.expect("delete");
    assert!(
        store
            .get_connector(connector.id)
            .await
            .expect("get")
            .is_none()
    );
}

#[tokio::test]
async fn capability_crud() {
    let store = test_store().await;
    let cred = test_credential();
    store.insert_credential(&cred).await.expect("insert cred");

    let now = Utc::now();
    let connector = Connector {
        id: Uuid::new_v4(),
        name: "test-conn".into(),
        provider: "openai".into(),
        credential_id: cred.id,
        base_url: None,
        enabled: true,
        created_at: now,
        updated_at: now,
    };
    store
        .insert_connector(&connector)
        .await
        .expect("insert conn");

    let cap = Capability {
        id: Uuid::new_v4(),
        connector_id: connector.id,
        name: "openai.chat".into(),
        description: Some("Chat completions".into()),
        enabled: true,
        created_at: now,
    };
    store.insert_capability(&cap).await.expect("insert cap");

    let fetched = store
        .get_capability_by_name(connector.id, "openai.chat")
        .await
        .expect("get")
        .expect("found");
    assert_eq!(fetched.id, cap.id);

    let list = store
        .list_capabilities_for_connector(connector.id)
        .await
        .expect("list");
    assert_eq!(list.len(), 1);

    store.delete_capability(cap.id).await.expect("delete");
    assert!(store.get_capability(cap.id).await.expect("get").is_none());
}

#[tokio::test]
async fn grant_crud() {
    let store = test_store().await;
    let cred = test_credential();
    store.insert_credential(&cred).await.expect("insert cred");

    let now = Utc::now();
    let connector = Connector {
        id: Uuid::new_v4(),
        name: "g-conn".into(),
        provider: "openai".into(),
        credential_id: cred.id,
        base_url: None,
        enabled: true,
        created_at: now,
        updated_at: now,
    };
    store
        .insert_connector(&connector)
        .await
        .expect("insert conn");

    let cap = Capability {
        id: Uuid::new_v4(),
        connector_id: connector.id,
        name: "openai.*".into(),
        description: None,
        enabled: true,
        created_at: now,
    };
    store.insert_capability(&cap).await.expect("insert cap");

    let grant = Grant {
        id: Uuid::new_v4(),
        agent_name: "claude".into(),
        capability_id: cap.id,
        ttl_minutes: Some(60),
        max_requests: Some(100),
        require_confirmation: false,
        enabled: true,
        created_at: now,
        expires_at: None,
    };
    store.insert_grant(&grant).await.expect("insert grant");

    let list = store
        .list_grants_for_agent("claude")
        .await
        .expect("list by agent");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].ttl_minutes, Some(60));

    let list_cap = store
        .list_grants_for_capability(cap.id)
        .await
        .expect("list by cap");
    assert_eq!(list_cap.len(), 1);

    store
        .set_grant_enabled(grant.id, false)
        .await
        .expect("disable");
    let disabled = store
        .get_grant(grant.id)
        .await
        .expect("get")
        .expect("found");
    assert!(!disabled.enabled);

    store.delete_grant(grant.id).await.expect("delete");
    assert!(store.get_grant(grant.id).await.expect("get").is_none());
}

#[tokio::test]
async fn bundle_with_grants() {
    let store = test_store().await;
    let cred = test_credential();
    store.insert_credential(&cred).await.expect("insert cred");

    let now = Utc::now();
    let connector = Connector {
        id: Uuid::new_v4(),
        name: "b-conn".into(),
        provider: "openai".into(),
        credential_id: cred.id,
        base_url: None,
        enabled: true,
        created_at: now,
        updated_at: now,
    };
    store
        .insert_connector(&connector)
        .await
        .expect("insert conn");

    let cap = Capability {
        id: Uuid::new_v4(),
        connector_id: connector.id,
        name: "openai.*".into(),
        description: None,
        enabled: true,
        created_at: now,
    };
    store.insert_capability(&cap).await.expect("insert cap");

    let grant = Grant {
        id: Uuid::new_v4(),
        agent_name: "*".into(),
        capability_id: cap.id,
        ttl_minutes: None,
        max_requests: None,
        require_confirmation: false,
        enabled: true,
        created_at: now,
        expires_at: None,
    };
    store.insert_grant(&grant).await.expect("insert grant");

    let bundle = Bundle {
        id: Uuid::new_v4(),
        name: "dev-bundle".into(),
        description: Some("Development".into()),
        source_profile_id: None,
        created_at: now,
        updated_at: now,
    };
    store.insert_bundle(&bundle).await.expect("insert bundle");

    store
        .add_grant_to_bundle(bundle.id, grant.id)
        .await
        .expect("add grant");

    let grants = store
        .list_grants_for_bundle(bundle.id)
        .await
        .expect("list grants");
    assert_eq!(grants.len(), 1);
    assert_eq!(grants[0].id, grant.id);

    let fetched = store
        .get_bundle_by_name("dev-bundle")
        .await
        .expect("get")
        .expect("found");
    assert_eq!(fetched.id, bundle.id);

    store.delete_bundle(bundle.id).await.expect("delete bundle");
    assert!(store.get_bundle(bundle.id).await.expect("get").is_none());
}

#[tokio::test]
async fn session_crud() {
    let store = test_store().await;

    let now = Utc::now();
    let session = Session {
        id: Uuid::new_v4(),
        bundle_id: None,
        agent_name: "test-agent".into(),
        project: Some("proj".into()),
        issued_at: now,
        expires_at: now + chrono::Duration::minutes(60),
        session_token_hash: "abcdef1234567890".into(),
        request_count: 0,
    };
    store
        .insert_session(&session)
        .await
        .expect("insert session");

    let fetched = store
        .get_session(session.id)
        .await
        .expect("get")
        .expect("found");
    assert_eq!(fetched.agent_name, "test-agent");
    assert_eq!(fetched.request_count, 0);

    let by_hash = store
        .get_broker_session_by_token_hash("abcdef1234567890")
        .await
        .expect("get by hash")
        .expect("found");
    assert_eq!(by_hash.id, session.id);

    store
        .increment_session_request_count(session.id)
        .await
        .expect("increment");
    let updated = store
        .get_session(session.id)
        .await
        .expect("get")
        .expect("found");
    assert_eq!(updated.request_count, 1);

    let active = store.list_active_sessions().await.expect("list active");
    assert_eq!(active.len(), 1);

    store
        .delete_session(session.id)
        .await
        .expect("delete session");
    assert!(store.get_session(session.id).await.expect("get").is_none());
}

// --- Phase 1.1: candidate listing + per-grant quota counter ---------------

/// Helper to wire up the minimum graph needed for `list_session_proxy_candidates`.
async fn seed_session_with_grants(
    store: &Store,
    provider: &str,
    connector_name: &str,
    capability_name: &str,
    agent_name: &str,
    max_requests: Option<i64>,
) -> (Uuid, Uuid) {
    let cred = test_credential();
    store.insert_credential(&cred).await.expect("insert cred");

    let now = Utc::now();
    let connector = Connector {
        id: Uuid::new_v4(),
        name: connector_name.into(),
        provider: provider.into(),
        credential_id: cred.id,
        base_url: None,
        enabled: true,
        created_at: now,
        updated_at: now,
    };
    store
        .insert_connector(&connector)
        .await
        .expect("insert conn");

    let cap = Capability {
        id: Uuid::new_v4(),
        connector_id: connector.id,
        name: capability_name.into(),
        description: None,
        enabled: true,
        created_at: now,
    };
    store.insert_capability(&cap).await.expect("insert cap");

    let grant = Grant {
        id: Uuid::new_v4(),
        agent_name: agent_name.into(),
        capability_id: cap.id,
        ttl_minutes: None,
        max_requests,
        require_confirmation: false,
        enabled: true,
        created_at: now,
        expires_at: None,
    };
    store.insert_grant(&grant).await.expect("insert grant");

    let bundle = Bundle {
        id: Uuid::new_v4(),
        name: format!("b-{connector_name}"),
        description: None,
        source_profile_id: None,
        created_at: now,
        updated_at: now,
    };
    store.insert_bundle(&bundle).await.expect("insert bundle");
    store
        .add_grant_to_bundle(bundle.id, grant.id)
        .await
        .expect("add grant to bundle");

    let session = Session {
        id: Uuid::new_v4(),
        bundle_id: Some(bundle.id),
        agent_name: agent_name.into(),
        project: None,
        issued_at: now,
        expires_at: now + chrono::Duration::minutes(60),
        session_token_hash: format!("hash-{}", Uuid::new_v4()),
        request_count: 0,
    };
    store
        .insert_session_with_grants(&session, &[grant.id])
        .await
        .expect("insert session with grants");

    (session.id, grant.id)
}

#[tokio::test]
async fn list_session_proxy_candidates_returns_expected_rows() {
    let store = test_store().await;
    let (session_id, grant_id) = seed_session_with_grants(
        &store,
        "openai",
        "my-openai",
        "openai.chat",
        "claude",
        Some(5),
    )
    .await;

    let candidates = store
        .list_session_proxy_candidates(session_id, "openai")
        .await
        .expect("list candidates");
    assert_eq!(candidates.len(), 1);
    let c = &candidates[0];
    assert_eq!(c.grant.id, grant_id);
    assert_eq!(c.connector.name, "my-openai");
    assert_eq!(c.capability.name, "openai.chat");
    assert_eq!(c.session_grant_request_count, 0);
    assert_eq!(c.grant.max_requests, Some(5));

    // Provider mismatch returns zero candidates.
    let empty = store
        .list_session_proxy_candidates(session_id, "anthropic")
        .await
        .expect("list candidates anthropic");
    assert!(empty.is_empty());
}

#[tokio::test]
async fn increment_session_grant_request_count_bumps_counter() {
    let store = test_store().await;
    let (session_id, grant_id) = seed_session_with_grants(
        &store,
        "openai",
        "my-openai",
        "openai.chat",
        "claude",
        Some(3),
    )
    .await;

    store
        .increment_session_grant_request_count(session_id, grant_id)
        .await
        .expect("increment");
    store
        .increment_session_grant_request_count(session_id, grant_id)
        .await
        .expect("increment again");

    let candidates = store
        .list_session_proxy_candidates(session_id, "openai")
        .await
        .expect("list");
    assert_eq!(candidates[0].session_grant_request_count, 2);
}

#[tokio::test]
async fn list_session_proxy_candidates_excludes_disabled_cascade() {
    let store = test_store().await;
    let (session_id, grant_id) =
        seed_session_with_grants(&store, "openai", "my-openai", "openai.chat", "claude", None)
            .await;

    // Disable the grant — SQL filter should drop it.
    store
        .set_grant_enabled(grant_id, false)
        .await
        .expect("disable grant");
    let empty = store
        .list_session_proxy_candidates(session_id, "openai")
        .await
        .expect("list");
    assert!(empty.is_empty());
}
