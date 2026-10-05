use sqlx::SqlitePool;

/// State shared by every request handler.
#[derive(Clone)]
pub struct AppState {
    /// Database connection pool.
    pub pool: SqlitePool,
}
