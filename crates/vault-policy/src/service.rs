use anyhow::{Result, bail};
use chrono::Utc;
use vault_core::models::Grant;

#[derive(Debug, Clone, Default)]
pub struct PolicyService {
    pub allow_prod: bool,
}

impl PolicyService {
    pub fn ensure_environment_allowed(&self, environment: &str) -> Result<()> {
        if environment == "prod" && !self.allow_prod {
            bail!("prod credentials are blocked unless explicitly allowed");
        }
        Ok(())
    }

    pub fn check_grant_active(&self, grant: &Grant) -> Result<()> {
        if !grant.enabled {
            bail!("grant is disabled");
        }
        if let Some(expires_at) = grant.expires_at
            && expires_at < Utc::now()
        {
            bail!("grant has expired");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn policy() -> PolicyService {
        PolicyService::default()
    }

    fn active_grant() -> Grant {
        Grant {
            id: uuid::Uuid::new_v4(),
            agent_name: "test".into(),
            capability_id: uuid::Uuid::new_v4(),
            ttl_minutes: None,
            max_requests: None,
            require_confirmation: false,
            enabled: true,
            created_at: Utc::now(),
            expires_at: None,
        }
    }

    #[test]
    fn active_grant_passes() {
        assert!(policy().check_grant_active(&active_grant()).is_ok());
    }

    #[test]
    fn disabled_grant_fails() {
        let mut g = active_grant();
        g.enabled = false;
        let err = policy().check_grant_active(&g).unwrap_err();
        assert!(err.to_string().contains("disabled"));
    }

    #[test]
    fn expired_grant_fails() {
        let mut g = active_grant();
        g.expires_at = Some(Utc::now() - Duration::hours(1));
        let err = policy().check_grant_active(&g).unwrap_err();
        assert!(err.to_string().contains("expired"));
    }

    #[test]
    fn future_expiry_passes() {
        let mut g = active_grant();
        g.expires_at = Some(Utc::now() + Duration::hours(1));
        assert!(policy().check_grant_active(&g).is_ok());
    }
}
