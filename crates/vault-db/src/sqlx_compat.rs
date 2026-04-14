use sqlx_core::{
    database::Database, from_row::FromRow, query::Query, query_as::QueryAs,
    query_scalar::QueryScalar,
};
use sqlx_sqlite::Sqlite;

type SqliteArguments<'q> = <Sqlite as Database>::Arguments<'q>;

pub use sqlx_core::row::Row;
pub use sqlx_sqlite::SqlitePool;

pub fn query(sql: &str) -> Query<'_, Sqlite, SqliteArguments<'_>> {
    sqlx_core::query::query::<Sqlite>(sql)
}

pub fn query_as<'q, O>(sql: &'q str) -> QueryAs<'q, Sqlite, O, SqliteArguments<'q>>
where
    O: Send + Unpin + for<'r> FromRow<'r, sqlx_sqlite::SqliteRow>,
{
    sqlx_core::query_as::query_as::<Sqlite, O>(sql)
}

pub fn query_scalar<'q, O>(sql: &'q str) -> QueryScalar<'q, Sqlite, O, SqliteArguments<'q>>
where
    O: Send + Unpin,
    (O,): Send + Unpin + for<'r> FromRow<'r, sqlx_sqlite::SqliteRow>,
{
    sqlx_core::query_scalar::query_scalar::<Sqlite, O>(sql)
}

pub mod migrate {
    pub use sqlx_core::migrate::{Migration, MigrationType, Migrator};
}

pub mod sqlite {
    pub use sqlx_sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteRow};
}
