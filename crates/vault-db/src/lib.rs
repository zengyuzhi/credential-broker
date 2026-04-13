mod codec;

pub mod bindings;
pub mod credentials;
pub mod leases;
pub mod profiles;
pub mod store;

pub const DEVELOPMENT_DATABASE_URL: &str = "sqlite:.local/vault.db";

pub use store::Store;
