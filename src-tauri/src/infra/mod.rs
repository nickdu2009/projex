//! Infrastructure: SQLite connection, migrations, repositories.

pub mod db;

pub use db::{init_db, DbPool};
pub(crate) use db::get_connection;
