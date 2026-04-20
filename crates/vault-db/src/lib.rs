mod codec;
mod sqlx_compat;

pub use sqlx_compat::{Row, SqlitePool, migrate, query, query_as, query_scalar, sqlite};

extern crate self as sqlx;

pub mod bindings;
pub mod bundles;
pub mod capabilities;
pub mod connectors;
pub mod credentials;
pub mod grants;
pub mod leases;
pub mod profiles;
pub mod sessions;
pub mod store;
pub mod ui_sessions;
pub mod usage_events;

pub const DEVELOPMENT_DATABASE_URL: &str = "sqlite:.local/vault.db";

pub use store::Store;
pub use ui_sessions::UiSession;
pub use usage_events::ProviderStats;
