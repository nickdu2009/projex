//! SQLite connection and migrations.

use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

pub struct DbPool(pub Mutex<Connection>);

/// Initialize DB at path, run migrations, return managed pool.
pub fn init_db(db_path: &Path) -> Result<DbPool, crate::error::AppError> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| crate::error::AppError::Db(e.to_string()))?;
    }
    let mut conn = Connection::open(db_path).map_err(|e| crate::error::AppError::Db(e.to_string()))?;
    run_migrations(&mut conn)?;
    Ok(DbPool(Mutex::new(conn)))
}

fn run_migrations(conn: &mut Connection) -> Result<(), crate::error::AppError> {
    let tx = conn
        .transaction()
        .map_err(|e| crate::error::AppError::Db(e.to_string()))?;

    // Ensure schema_migrations exists (first run)
    tx.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT (datetime('now')))",
        [],
    )
    .map_err(|e| crate::error::AppError::Db(e.to_string()))?;

    let applied: Vec<i32> = tx
        .prepare("SELECT version FROM schema_migrations ORDER BY version")
        .map_err(|e| crate::error::AppError::Db(e.to_string()))?
        .query_map([], |r| r.get(0))
        .map_err(|e| crate::error::AppError::Db(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| crate::error::AppError::Db(e.to_string()))?;

    const MIGRATIONS: &[(i32, &str)] = &[
        (1, include_str!("../../migrations/0001_init.sql")),
        (2, include_str!("../../migrations/0002_add_person_email_role.sql")),
    ];

    for (version, sql) in MIGRATIONS {
        if applied.contains(version) {
            continue;
        }
        // Skip the INSERT into schema_migrations in the script - we do it ourselves
        let statements: Vec<&str> = sql
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && !s.contains("INSERT INTO schema_migrations"))
            .collect();
        for stmt in statements {
            if !stmt.is_empty() {
                tx.execute(stmt, [])
                    .map_err(|e| crate::error::AppError::Db(e.to_string()))?;
            }
        }
        tx.execute("INSERT INTO schema_migrations (version, applied_at) VALUES (?1, datetime('now'))", [version])
            .map_err(|e| crate::error::AppError::Db(e.to_string()))?;
    }

    tx.commit().map_err(|e| crate::error::AppError::Db(e.to_string()))?;
    Ok(())
}

/// Get connection from pool (for use in commands).
pub fn get_connection(pool: &DbPool) -> std::sync::MutexGuard<'_, Connection> {
    pool.0.lock().expect("db lock")
}
