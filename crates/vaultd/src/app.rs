use vault_db::Store;

use crate::auth::RateLimiter;

#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub http_client: reqwest::Client,
    pub rate_limiter: RateLimiter,
}

impl AppState {
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        let store = Store::connect(database_url).await?;
        let http_client = reqwest::Client::new();
        let rate_limiter = RateLimiter::new();
        Ok(Self {
            store,
            http_client,
            rate_limiter,
        })
    }
}
