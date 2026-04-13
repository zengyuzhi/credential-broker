use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use vault_core::provider::{ParsedUsage, ProviderAdapter, ResolvedCredential};

use crate::generic::{require_field, single_env};

#[derive(Debug, Default)]
pub struct TwitterApiAdapter;

#[async_trait]
impl ProviderAdapter for TwitterApiAdapter {
    fn provider_id(&self) -> &'static str {
        "twitterapi"
    }

    fn supports_inject(&self) -> bool {
        true
    }

    fn supports_proxy(&self) -> bool {
        true
    }

    fn env_map(&self, resolved: &ResolvedCredential) -> Result<HashMap<String, String>> {
        Ok(single_env(
            "TWITTERAPI_API_KEY",
            require_field(resolved, "api_key")?,
        ))
    }

    fn upstream_base_url(&self) -> Option<&'static str> {
        Some("https://api.twitterapi.io")
    }

    fn parse_usage_from_response(
        &self,
        endpoint: &str,
        _status_code: u16,
        _response_body: &[u8],
    ) -> ParsedUsage {
        ParsedUsage {
            operation: "http_request".to_string(),
            endpoint: Some(endpoint.to_string()),
            model: None,
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
            estimated_cost_usd: None,
        }
    }
}
