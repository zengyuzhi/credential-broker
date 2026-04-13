use std::sync::Arc;

use anyhow::{Result, anyhow};
use vault_core::provider::ProviderAdapter;

use crate::{anthropic::AnthropicAdapter, openai::OpenAiAdapter, twitterapi::TwitterApiAdapter};

pub fn adapter_for(provider: &str) -> Result<Arc<dyn ProviderAdapter>> {
    match provider {
        "openai" => Ok(Arc::new(OpenAiAdapter)),
        "anthropic" => Ok(Arc::new(AnthropicAdapter)),
        "twitterapi" => Ok(Arc::new(TwitterApiAdapter)),
        other => Err(anyhow!("unsupported provider: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use vault_core::provider::ResolvedCredential;

    use crate::schema_for;

    use super::adapter_for;

    #[test]
    fn openai_adapter_maps_api_key_env() {
        let adapter = adapter_for("openai").expect("openai adapter");
        let resolved = ResolvedCredential {
            provider: "openai".to_string(),
            label: "test".to_string(),
            fields: HashMap::from([("api_key".to_string(), "secret".to_string())]),
        };
        let env = adapter.env_map(&resolved).expect("env mapping");
        assert_eq!(
            env.get("OPENAI_API_KEY").map(String::as_str),
            Some("secret")
        );
    }

    #[test]
    fn provider_schema_should_describe_phase1_supported_fields() {
        let openai = schema_for("openai").expect("openai schema");
        let github = schema_for("github").expect("github schema");
        let tavily = schema_for("tavily").expect("tavily schema");

        assert_eq!(openai.required_fields, &["api_key"]);
        assert_eq!(github.required_fields, &["token"]);
        assert_eq!(tavily.required_fields, &["api_key"]);
        assert!(schema_for("coingecko").is_some());
        assert!(schema_for("openrouter").is_some());
        assert!(schema_for("twitterapi").is_some());
        assert!(schema_for("anthropic").is_some());
    }
}
