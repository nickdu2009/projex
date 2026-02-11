//! Infrastructure: SQLite connection, migrations, repositories.

pub mod db;

pub(crate) use db::get_connection;
pub use db::{init_db, DbPool};
