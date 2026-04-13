#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    eprintln!("Note: standalone vaultd is deprecated. Use `vault serve` instead.");

    let database_url = std::env::var("VAULT_DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:.local/vault.db".to_string());
    vaultd::start_server(&database_url, 8765).await
}
