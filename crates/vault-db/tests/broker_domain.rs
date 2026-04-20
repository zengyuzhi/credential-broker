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
