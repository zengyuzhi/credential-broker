use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use vault_core::models::{AccessMode, CredentialKind};

pub fn credential_kind_as_str(kind: &CredentialKind) -> &'static str {
    match kind {
        CredentialKind::ApiKey => "api_key",
        CredentialKind::BearerToken => "bearer_token",
        CredentialKind::OAuth => "oauth",
        CredentialKind::Bundle => "bundle",
    }
}

pub fn parse_credential_kind(value: &str) -> Result<CredentialKind> {
    match value {
        "api_key" => Ok(CredentialKind::ApiKey),
        "bearer_token" => Ok(CredentialKind::BearerToken),
        "oauth" => Ok(CredentialKind::OAuth),
        "bundle" => Ok(CredentialKind::Bundle),
        other => Err(anyhow!("unknown credential kind: {other}")),
    }
}

pub fn access_mode_as_str(mode: &AccessMode) -> &'static str {
    match mode {
        AccessMode::Inject => "inject",
        AccessMode::Proxy => "proxy",
        AccessMode::Either => "either",
    }
}

pub fn parse_access_mode(value: &str) -> Result<AccessMode> {
    match value {
        "inject" => Ok(AccessMode::Inject),
        "proxy" => Ok(AccessMode::Proxy),
        "either" => Ok(AccessMode::Either),
        other => Err(anyhow!("unknown access mode: {other}")),
    }
}

pub fn parse_timestamp(value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
}
