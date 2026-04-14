use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CredentialKind {
    ApiKey,
    BearerToken,
    OAuth,
    Bundle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AccessMode {
    Inject,
    Proxy,
    Either,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    pub id: Uuid,
    pub provider: String,
    pub kind: CredentialKind,
    pub label: String,
    pub secret_ref: String,
    pub environment: String,
    pub owner: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub default_project: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileBinding {
    pub id: Uuid,
    pub profile_id: Uuid,
    pub provider: String,
    pub credential_id: Uuid,
    pub mode: AccessMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lease {
    pub id: Uuid,
    pub profile_id: Uuid,
    pub agent_name: String,
    pub project: Option<String>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub session_token_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEvent {
    pub id: Uuid,
    pub provider: String,
    pub credential_id: Uuid,
    pub lease_id: Option<Uuid>,
    pub agent_name: String,
    pub project: Option<String>,
    pub mode: AccessMode,
    pub operation: String,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub request_count: i64,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    /// Cost in integer microdollars (1 microdollar = $0.000001). Audit SE-09.
    pub estimated_cost_micros: Option<i64>,
    pub status_code: Option<i64>,
    pub success: bool,
    pub latency_ms: i64,
    pub error_text: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_mode_serializes_as_json() {
        let mode = AccessMode::Proxy;
        let serialized = serde_json::to_string(&mode).expect("serialize mode");
        assert_eq!(serialized, "\"Proxy\"");
    }
}
