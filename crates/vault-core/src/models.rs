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

// --- Phase 1: Broker core domain model ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connector {
    pub id: Uuid,
    pub name: String,
    pub provider: String,
    pub credential_id: Uuid,
    pub base_url: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub id: Uuid,
    pub connector_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grant {
    pub id: Uuid,
    pub agent_name: String,
    pub capability_id: Uuid,
    pub ttl_minutes: Option<i64>,
    pub max_requests: Option<i64>,
    pub require_confirmation: bool,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bundle {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub source_profile_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub bundle_id: Option<Uuid>,
    pub agent_name: String,
    pub project: Option<String>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub session_token_hash: String,
    pub request_count: i64,
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

    #[test]
    fn connector_round_trip() {
        let c = Connector {
            id: Uuid::new_v4(),
            name: "my-openai".into(),
            provider: "openai".into(),
            credential_id: Uuid::new_v4(),
            base_url: Some("https://api.openai.com".into()),
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&c).expect("serialize");
        let d: Connector = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(c.id, d.id);
        assert_eq!(c.name, d.name);
    }

    #[test]
    fn capability_round_trip() {
        let c = Capability {
            id: Uuid::new_v4(),
            connector_id: Uuid::new_v4(),
            name: "openai.chat".into(),
            description: Some("Chat completions".into()),
            enabled: true,
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&c).expect("serialize");
        let d: Capability = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(c.id, d.id);
        assert_eq!(c.name, d.name);
    }

    #[test]
    fn grant_round_trip() {
        let g = Grant {
            id: Uuid::new_v4(),
            agent_name: "claude".into(),
            capability_id: Uuid::new_v4(),
            ttl_minutes: Some(60),
            max_requests: None,
            require_confirmation: false,
            enabled: true,
            created_at: Utc::now(),
            expires_at: None,
        };
        let json = serde_json::to_string(&g).expect("serialize");
        let d: Grant = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(g.id, d.id);
        assert_eq!(g.agent_name, d.agent_name);
    }

    #[test]
    fn bundle_round_trip() {
        let b = Bundle {
            id: Uuid::new_v4(),
            name: "dev-bundle".into(),
            description: None,
            source_profile_id: Some(Uuid::new_v4()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&b).expect("serialize");
        let d: Bundle = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(b.id, d.id);
        assert_eq!(b.source_profile_id, d.source_profile_id);
    }

    #[test]
    fn session_round_trip() {
        let s = Session {
            id: Uuid::new_v4(),
            bundle_id: Some(Uuid::new_v4()),
            agent_name: "test-agent".into(),
            project: Some("proj".into()),
            issued_at: Utc::now(),
            expires_at: Utc::now(),
            session_token_hash: "abc123".into(),
            request_count: 0,
        };
        let json = serde_json::to_string(&s).expect("serialize");
        let d: Session = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(s.id, d.id);
        assert_eq!(s.agent_name, d.agent_name);
        assert_eq!(s.request_count, d.request_count);
    }
}
