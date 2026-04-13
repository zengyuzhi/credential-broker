use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct ResolvedCredential {
    pub provider: String,
    pub label: String,
    pub fields: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct ParsedUsage {
    pub operation: String,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub estimated_cost_usd: Option<f64>,
}

#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    fn provider_id(&self) -> &'static str;
    fn supports_inject(&self) -> bool;
    fn supports_proxy(&self) -> bool;
    fn env_map(&self, resolved: &ResolvedCredential) -> Result<HashMap<String, String>>;
    fn upstream_base_url(&self) -> Option<&'static str>;
    fn parse_usage_from_response(
        &self,
        endpoint: &str,
        status_code: u16,
        response_body: &[u8],
    ) -> ParsedUsage;
}
