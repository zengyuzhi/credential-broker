use chrono::{Duration, Utc};
use uuid::Uuid;
use vault_core::models::Lease;

pub fn issue_lease(
    profile_id: Uuid,
    agent_name: &str,
    project: Option<String>,
    ttl_minutes: i64,
) -> (Lease, String) {
    let raw_token = format!("{}{}", Uuid::new_v4(), Uuid::new_v4());
    let issued_at = Utc::now();
    let lease = Lease {
        id: Uuid::new_v4(),
        profile_id,
        agent_name: agent_name.to_string(),
        project,
        issued_at,
        expires_at: issued_at + Duration::minutes(ttl_minutes),
        session_token_hash: hash_token(&raw_token),
    };
    (lease, raw_token)
}

pub fn hash_token(raw_token: &str) -> String {
    blake3::hash(raw_token.as_bytes()).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::hash_token;

    #[test]
    fn token_hash_is_stable() {
        let a = hash_token("abc");
        let b = hash_token("abc");
        assert_eq!(a, b);
    }
}
