use vault_db::{Store, query, query_as, query_scalar};

#[tokio::test]
async fn connect_should_open_sqlite_database() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_url = format!("sqlite:{}", dir.path().join("test.db").display());

    let store = Store::connect(&db_url).await.expect("connect store");
    let row: (i64,) = query_as("select 1")
        .fetch_one(&store.pool)
        .await
        .expect("select 1");
    let credentials_table: Option<String> = query_scalar(
        "select name from sqlite_master where type = 'table' and name = 'credentials'",
    )
    .fetch_optional(&store.pool)
    .await
    .expect("fetch credentials table");

    assert_eq!(row.0, 1);
    assert_eq!(credentials_table.as_deref(), Some("credentials"));
}

#[tokio::test]
async fn connect_creates_parent_directory_and_enforces_foreign_keys() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("nested/path/vault.db");
    let db_url = format!("sqlite:{}", db_path.display());

    let store = Store::connect(&db_url).await.expect("connect store");
    let foreign_key_error = query(
        "insert into credential_fields (credential_id, field_name, value_ref) values (?1, ?2, ?3)",
    )
    .bind("missing-credential")
    .bind("api_key")
    .bind("ref")
    .execute(&store.pool)
    .await
    .expect_err("foreign keys should reject missing credential");

    assert!(db_path.exists());
    assert!(foreign_key_error.to_string().contains("FOREIGN KEY"));
}

#[tokio::test]
async fn connect_supports_triple_slash_sqlite_urls_for_absolute_paths() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("absolute.db");
    let db_url = format!("sqlite://{}", db_path.display());

    let store = Store::connect(&db_url).await.expect("connect store");
    let count: i64 = query_scalar(
        "select count(*) from sqlite_master where type = 'table' and name = 'credentials'",
    )
    .fetch_one(&store.pool)
    .await
    .expect("count credentials table");

    assert_eq!(count, 1);
    assert!(db_path.exists());
}

#[tokio::test]
async fn connect_supports_in_memory_databases() {
    let store = Store::connect("sqlite::memory:")
        .await
        .expect("connect in-memory store");
    let count: i64 = query_scalar(
        "select count(*) from sqlite_master where type = 'table' and name = 'credentials'",
    )
    .fetch_one(&store.pool)
    .await
    .expect("count credentials table");

    assert_eq!(count, 1);
}
