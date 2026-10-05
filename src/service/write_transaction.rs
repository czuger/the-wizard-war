use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::error::AppError;

/// Starts a transaction that takes the write lock at once.
///
/// Two players acting at the same moment would otherwise both read the same state and
/// one of them would fail when upgrading to a write lock.
pub async fn begin_write(pool: &SqlitePool) -> Result<Transaction<'static, Sqlite>, AppError> {
    Ok(pool.begin_with("BEGIN IMMEDIATE").await?)
}
