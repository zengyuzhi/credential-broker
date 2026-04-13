use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use vault_core::provider::{ParsedUsage, ProviderAdapter, ResolvedCredential};

use crate::generic::{require_field, single_env};

#[derive(Debug, Default)]
pub struct AnthropicAdapter;

#[async_trait]
impl ProviderAdapter for AnthropicAdapter {
    fn provider_id(&self) -> &'static str {
        "anthropic"
    }

    fn supports_inject(&self) -> bool {
        true
    }

    fn supports_proxy(&self) -> bool {
        true
    }

    fn env_map(&self, resolved: &ResolvedCredential) -> Result<HashMap<String, String>> {
        Ok(single_env(
            "ANTHROPIC_API_KEY",
            require_field(resolved, "api_key")?,
        ))
    }

    fn upstream_base_url(&self) -> Option<&'static str> {
        Some("https://api.anthropic.com")
    }

    fn parse_usage_from_response(
        &self,
        endpoint: &str,
        _status_code: u16,
        response_body: &[u8],
    ) -> ParsedUsage {
        let parsed: Value = serde_json::from_slice(response_body).unwrap_or(Value::Null);
        let usage = parsed.get("usage").cloned().unwrap_or(Value::Null);
        let input = usage.get("input_tokens").and_then(Value::as_i64);
        let output = usage.get("output_tokens").and_then(Value::as_i64);
        ParsedUsage {
            operation: "request".to_string(),
            endpoint: Some(endpoint.to_string()),
            model: parsed
                .get("model")
                .and_then(Value::as_str)
                .map(str::to_string),
            prompt_tokens: input,
            completion_tokens: output,
            total_tokens: match (input, output) {
                (Some(i), Some(o)) => Some(i + o),
                _ => None,
            },
            estimated_cost_usd: None,
        }
    }
}
