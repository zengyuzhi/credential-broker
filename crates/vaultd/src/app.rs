use vault_db::Store;

#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub http_client: reqwest::Client,
}

impl AppState {
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        let store = Store::connect(database_url).await?;
        let http_client = reqwest::Client::new();
        Ok(Self { store, http_client })
    }
}
