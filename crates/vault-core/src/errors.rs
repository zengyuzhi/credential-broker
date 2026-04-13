use thiserror::Error;

#[derive(Debug, Error)]
pub enum VaultError {
    #[error("unsupported provider: {0}")]
    UnsupportedProvider(String),
    #[error("missing credential field: {0}")]
    MissingCredentialField(&'static str),
}
