use anyhow::{Result, bail};

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
}
