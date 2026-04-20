use std::num::NonZeroU32;

use chrono::{Duration, Utc};
use uuid::Uuid;
use vault_core::models::Session;
use zeroize::Zeroizing;

use crate::lease::hash_token;

pub fn issue_session(
    bundle_id: Option<Uuid>,
    agent_name: &str,
    project: Option<String>,
    ttl_minutes: NonZeroU32,
) -> (Session, Zeroizing<String>) {
    let raw_token = Zeroizing::new(format!("{}{}", Uuid::new_v4(), Uuid::new_v4()));
    let issued_at = Utc::now();
    let session = Session {
        id: Uuid::new_v4(),
        bundle_id,
        agent_name: agent_name.to_string(),
        project,
        issued_at,
        expires_at: issued_at + Duration::minutes(i64::from(ttl_minutes.get())),
        session_token_hash: hash_token(&raw_token),
        request_count: 0,
    };
    (session, raw_token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_session_produces_valid_token() {
        let ttl = NonZeroU32::new(30).expect("30 is nonzero");
        let (session, raw_token) = issue_session(None, "test-agent", None, ttl);

        assert!(!raw_token.is_empty());
        assert_eq!(session.session_token_hash, hash_token(&raw_token));
        assert_eq!(session.agent_name, "test-agent");
        assert_eq!(session.request_count, 0);
        assert!(session.expires_at > session.issued_at);
    }

    #[test]
    fn issue_session_with_bundle() {
        let bundle_id = Uuid::new_v4();
        let ttl = NonZeroU32::new(60).expect("60 is nonzero");
        let (session, _) = issue_session(Some(bundle_id), "agent", Some("proj".to_string()), ttl);

        assert_eq!(session.bundle_id, Some(bundle_id));
        assert_eq!(session.project.as_deref(), Some("proj"));
    }
}
