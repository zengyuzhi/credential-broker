mod app;
mod routes;

use std::net::SocketAddr;

use axum::Router;
use routes::{health::health_router, stats::stats_router};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let app = Router::new().merge(health_router()).merge(stats_router());

    let addr = SocketAddr::from(([127, 0, 0, 1], 8765));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("vaultd listening on http://{}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
