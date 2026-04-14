use std::num::NonZeroU32;

use chrono::{Duration, Utc};
use uuid::Uuid;
use vault_core::models::Lease;
use zeroize::Zeroizing;

/// Issue a new lease for `profile_id`.
///
/// - `ttl_minutes: NonZeroU32` enforces at the type level that zero, negative,
///   and `i64::MAX`-sized TTL values cannot reach this function (audit SE-07).
/// - The raw token is returned wrapped in `Zeroizing<String>` so the primary
///   heap allocation is wiped on drop (audit ZA-0004).
pub fn issue_lease(
    profile_id: Uuid,
    agent_name: &str,
    project: Option<String>,
    ttl_minutes: NonZeroU32,
) -> (Lease, Zeroizing<String>) {
    let raw_token = Zeroizing::new(format!("{}{}", Uuid::new_v4(), Uuid::new_v4()));
    let issued_at = Utc::now();
    let lease = Lease {
        id: Uuid::new_v4(),
        profile_id,
        agent_name: agent_name.to_string(),
        project,
        issued_at,
        expires_at: issued_at + Duration::minutes(i64::from(ttl_minutes.get())),
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
