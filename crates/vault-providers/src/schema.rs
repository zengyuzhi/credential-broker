use vault_core::models::AccessMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderSchema {
    pub provider: &'static str,
    pub required_fields: &'static [&'static str],
    pub default_mode: AccessMode,
}

const OPENAI_SCHEMA: ProviderSchema = ProviderSchema {
    provider: "openai",
    required_fields: &["api_key"],
    default_mode: AccessMode::Either,
};

const ANTHROPIC_SCHEMA: ProviderSchema = ProviderSchema {
    provider: "anthropic",
    required_fields: &["api_key"],
    default_mode: AccessMode::Either,
};

const OPENROUTER_SCHEMA: ProviderSchema = ProviderSchema {
    provider: "openrouter",
    required_fields: &["api_key"],
    default_mode: AccessMode::Either,
};

const TWITTERAPI_SCHEMA: ProviderSchema = ProviderSchema {
    provider: "twitterapi",
    required_fields: &["api_key"],
    default_mode: AccessMode::Either,
};

const GITHUB_SCHEMA: ProviderSchema = ProviderSchema {
    provider: "github",
    required_fields: &["token"],
    default_mode: AccessMode::Inject,
};

const TAVILY_SCHEMA: ProviderSchema = ProviderSchema {
    provider: "tavily",
    required_fields: &["api_key"],
    default_mode: AccessMode::Inject,
};

const COINGECKO_SCHEMA: ProviderSchema = ProviderSchema {
    provider: "coingecko",
    required_fields: &["api_key"],
    default_mode: AccessMode::Inject,
};

pub fn schema_for(provider: &str) -> Option<ProviderSchema> {
    match provider {
        "openai" => Some(OPENAI_SCHEMA),
        "anthropic" => Some(ANTHROPIC_SCHEMA),
        "openrouter" => Some(OPENROUTER_SCHEMA),
        "twitterapi" => Some(TWITTERAPI_SCHEMA),
        "github" => Some(GITHUB_SCHEMA),
        "tavily" => Some(TAVILY_SCHEMA),
        "coingecko" => Some(COINGECKO_SCHEMA),
        _ => None,
    }
}
