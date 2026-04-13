use chrono::Utc;
use tempfile::TempDir;
use uuid::Uuid;
use vault_core::models::{AccessMode, Credential, CredentialKind, Profile, ProfileBinding};
use vault_db::Store;

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
