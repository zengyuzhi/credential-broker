pub mod app;
pub mod auth;
pub mod routes;
pub mod static_assets;

use std::net::SocketAddr;

use app::AppState;

/// Start the vault HTTP server. Blocks until the server shuts down.
pub async fn start_server(database_url: &str, port: u16) -> anyhow::Result<()> {
    let state = AppState::new(database_url).await?;
    let app = routes::router(state);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Vault server listening on http://{}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
