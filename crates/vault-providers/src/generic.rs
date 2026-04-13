use std::collections::HashMap;

use anyhow::{Result, anyhow};
use vault_core::provider::ResolvedCredential;

pub fn require_field<'a>(resolved: &'a ResolvedCredential, field: &str) -> Result<&'a str> {
    resolved
        .fields
        .get(field)
        .map(|value| value.as_str())
        .ok_or_else(|| {
            anyhow!(
                "missing required field {field} for provider {}",
                resolved.provider
            )
        })
}

pub fn single_env(key: &str, value: &str) -> HashMap<String, String> {
    HashMap::from([(key.to_string(), value.to_string())])
}
