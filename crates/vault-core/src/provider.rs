use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use zeroize::Zeroize;

/// Carries a resolved secret value into adapter/env-map construction.
///
/// `fields` values are raw API keys. `zeroize` 1.x has no blanket `Zeroize`
/// impl for `HashMap` (the keys are metadata, not secrets, so a derive
/// doesn't fit), so we implement `Drop` by hand to wipe the value strings
/// while leaving the key strings intact. Audit ZA-0002 / ZA-0006.
#[derive(Debug, Clone)]
pub struct ResolvedCredential {
    pub provider: String,
    pub label: String,
    pub fields: HashMap<String, String>,
}

impl Drop for ResolvedCredential {
    fn drop(&mut self) {
        for value in self.fields.values_mut() {
            value.zeroize();
        }
    }
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
