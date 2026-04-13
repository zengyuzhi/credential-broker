use vault_core::models::UsageEvent;
use vault_db::Store;

#[derive(Clone)]
pub struct TelemetryWriter {
    pub store: Store,
}

impl TelemetryWriter {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    pub async fn write_usage_event(&self, _event: &UsageEvent) -> anyhow::Result<()> {
        Ok(())
    }
}
