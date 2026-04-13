mod app;
mod auth;
mod routes;

use std::net::SocketAddr;

use app::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let database_url = std::env::var("VAULT_DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:.local/vault.db".to_string());

    let state = AppState::new(&database_url).await?;

    let app = routes::router(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8765));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("vaultd listening on http://{}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
