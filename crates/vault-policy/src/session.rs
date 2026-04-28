use std::fmt::Write as _;
use std::num::NonZeroU32;

use anyhow::{Result, bail};
use chrono::{Duration, Utc};
use uuid::Uuid;
use vault_core::models::{Grant, Session};
use zeroize::Zeroizing;

use crate::{lease::hash_token, service::PolicyService};

/// Verify the session has not yet expired. Callers should also pass the
/// linked grant through `PolicyService::check_grant_active` for defense in
/// depth, and check per-request quotas (e.g. `grant.max_requests`) separately.
pub fn check_session_active(session: &Session) -> Result<()> {
    if session.expires_at < Utc::now() {
        bail!("session has expired");
    }
    Ok(())
}

pub fn issue_session(
    bundle_id: Option<Uuid>,
    agent_name: &str,
    project: Option<String>,
    ttl_minutes: NonZeroU32,
) -> (Session, Zeroizing<String>) {
    // Pre-allocate the full 72-byte token buffer so `write!` never reallocates
    // and leaves un-zeroized fragments in the allocator's free list. UUIDs
    // format into a stack buffer, then copy into the Zeroizing<String> via
    // push_str — no intermediate heap String is created.
    let mut raw_token = Zeroizing::new(String::with_capacity(72));
    write!(raw_token, "{}{}", Uuid::new_v4(), Uuid::new_v4())
        .expect("write! into String is infallible");
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

/// Phase 1.1: result of filtering bundle grants for a session issuance.
/// `attachable` grants are passed to the session; `skipped_confirmation`
/// are reported back to the operator but NOT attached (interactive
/// confirmation isn't wired yet).
#[derive(Debug, Default)]
pub struct GrantSelection<'a> {
    pub attachable: Vec<&'a Grant>,
    pub skipped_confirmation: Vec<&'a Grant>,
}

/// Phase 1.1: pick the subset of `grants` that can be attached to a session
/// issued to `agent_name`. Applies three filters in order:
///
/// 1. `PolicyService::check_grant_active` — enabled and not expired.
/// 2. Agent scope — `grant.agent_name == agent_name` or `"*"` wildcard.
///    Prevents cross-agent privilege leaks where a `codex` session
///    inherits a `claude`-only grant from the same bundle.
/// 3. `require_confirmation == false` — interactive approval isn't wired
///    yet; such grants are reported via `skipped_confirmation` so the
///    operator can notice.
pub fn select_attachable_grants<'a>(grants: &'a [Grant], agent_name: &str) -> GrantSelection<'a> {
    let policy = PolicyService::default();
    let mut selection = GrantSelection::default();
    for grant in grants {
        if policy.check_grant_active(grant).is_err() {
            continue;
        }
        if grant.agent_name != agent_name && grant.agent_name != "*" {
            continue;
        }
        if grant.require_confirmation {
            selection.skipped_confirmation.push(grant);
        } else {
            selection.attachable.push(grant);
        }
    }
    selection
}

/// Phase 1.1: clamp a user-supplied session TTL down to the tightest
/// `grant.ttl_minutes` across attached grants. Returns the clamped TTL
/// and, if clamping happened, the cap value the operator should see in
/// a warning. Non-positive grant caps are ignored (they would produce
/// an unusable zero-length session).
pub fn clamp_session_ttl(
    user_ttl: NonZeroU32,
    attached_grants: &[&Grant],
) -> (NonZeroU32, Option<u32>) {
    let grant_cap = attached_grants
        .iter()
        .filter_map(|g| g.ttl_minutes)
        .filter_map(|m| u32::try_from(m).ok())
        .filter(|m| *m > 0)
        .min();
    match grant_cap {
        Some(cap) if cap < user_ttl.get() => {
            (NonZeroU32::new(cap).expect("cap > 0 by filter"), Some(cap))
        }
        _ => (user_ttl, None),
    }
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

    #[test]
    fn check_session_active_passes_for_future_expiry() {
        let ttl = NonZeroU32::new(30).expect("30 is nonzero");
        let (session, _) = issue_session(None, "agent", None, ttl);
        assert!(check_session_active(&session).is_ok());
    }

    #[test]
    fn check_session_active_fails_for_past_expiry() {
        let ttl = NonZeroU32::new(1).expect("1 is nonzero");
        let (mut session, _) = issue_session(None, "agent", None, ttl);
        session.expires_at = Utc::now() - Duration::minutes(5);
        let err = check_session_active(&session).unwrap_err();
        assert!(err.to_string().contains("expired"));
    }

    // --- Phase 1.1: grant selection / TTL clamp -------------------------

    fn test_grant(agent: &str, require_confirmation: bool, ttl_minutes: Option<i64>) -> Grant {
        Grant {
            id: Uuid::new_v4(),
            agent_name: agent.to_string(),
            capability_id: Uuid::new_v4(),
            ttl_minutes,
            max_requests: None,
            require_confirmation,
            enabled: true,
            created_at: Utc::now(),
            expires_at: None,
        }
    }

    #[test]
    fn select_attachable_grants_filters_by_agent_name() {
        let grants = vec![
            test_grant("claude", false, None),
            test_grant("codex", false, None),
            test_grant("*", false, None),
        ];
        let sel = select_attachable_grants(&grants, "claude");
        assert_eq!(sel.attachable.len(), 2);
        let names: Vec<&str> = sel
            .attachable
            .iter()
            .map(|g| g.agent_name.as_str())
            .collect();
        assert!(names.contains(&"claude"));
        assert!(names.contains(&"*"));
        assert!(!names.contains(&"codex"));
        assert!(sel.skipped_confirmation.is_empty());
    }

    #[test]
    fn select_attachable_grants_partitions_confirmation_required() {
        let grants = vec![
            test_grant("claude", false, None),
            test_grant("claude", true, None),
        ];
        let sel = select_attachable_grants(&grants, "claude");
        assert_eq!(sel.attachable.len(), 1);
        assert_eq!(sel.skipped_confirmation.len(), 1);
        assert!(sel.skipped_confirmation[0].require_confirmation);
    }

    #[test]
    fn select_attachable_grants_excludes_disabled_and_expired() {
        let mut disabled = test_grant("claude", false, None);
        disabled.enabled = false;
        let mut expired = test_grant("claude", false, None);
        expired.expires_at = Some(Utc::now() - Duration::minutes(1));
        let live = test_grant("claude", false, None);

        let grants = vec![disabled, expired, live];
        let sel = select_attachable_grants(&grants, "claude");
        assert_eq!(sel.attachable.len(), 1);
    }

    #[test]
    fn clamp_session_ttl_reduces_to_tightest_grant_cap() {
        let user_ttl = NonZeroU32::new(10_080).expect("nonzero");
        let a = test_grant("claude", false, Some(120));
        let b = test_grant("claude", false, Some(60));
        let c = test_grant("claude", false, None);
        let attached = vec![&a, &b, &c];
        let (clamped, cap) = clamp_session_ttl(user_ttl, &attached);
        assert_eq!(clamped.get(), 60);
        assert_eq!(cap, Some(60));
    }

    #[test]
    fn clamp_session_ttl_leaves_user_ttl_untouched_when_no_cap() {
        let user_ttl = NonZeroU32::new(45).expect("nonzero");
        let a = test_grant("claude", false, None);
        let b = test_grant("claude", false, Some(60));
        let attached = vec![&a, &b];
        let (clamped, cap) = clamp_session_ttl(user_ttl, &attached);
        // User asked for 45 < grant cap of 60 → no clamp.
        assert_eq!(clamped.get(), 45);
        assert_eq!(cap, None);
    }

    #[test]
    fn clamp_session_ttl_ignores_non_positive_grant_caps() {
        let user_ttl = NonZeroU32::new(120).expect("nonzero");
        let a = test_grant("claude", false, Some(0));
        let b = test_grant("claude", false, Some(-5));
        let attached = vec![&a, &b];
        let (clamped, cap) = clamp_session_ttl(user_ttl, &attached);
        assert_eq!(clamped.get(), 120);
        assert_eq!(cap, None);
    }
}
