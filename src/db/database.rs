use std::str::FromStr;
use std::time::Duration;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

use crate::error::AppError;

/// Opens the SQLite database, creating the file if needed, and applies pending migrations.
///
/// # Errors
/// Fails when the URL is invalid, the file cannot be opened or a migration fails.
pub async fn connect(database_url: &str) -> Result<SqlitePool, AppError> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));
    // An in-memory database lives in one connection, so the pool must not open a second one.
    let max_connections = if database_url.contains(":memory:") {
        1
    } else {
        8
    };
    let pool = SqlitePoolOptions::new()
        .max_connections(max_connections)
        .idle_timeout(None)
        .max_lifetime(None)
        .connect_with(options)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
