use thiserror::Error;

#[derive(Debug, Error)]
pub enum VaultError {
    #[error("unsupported provider: {0}")]
    UnsupportedProvider(String),
    #[error("missing credential field: {0}")]
    MissingCredentialField(&'static str),
    #[error("timestamp arithmetic overflow in {0}")]
    TimestampOverflow(&'static str),
}
