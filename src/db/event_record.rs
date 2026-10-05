use sqlx::{FromRow, SqliteConnection};

use crate::error::AppError;

/// A line of the game log, shown to every player.
#[derive(Debug, Clone, FromRow)]
pub struct EventRecord {
    /// Turn during which the event happened.
    pub turn: i64,
    /// French text shown to players.
    pub message: String,
}

impl EventRecord {
    /// Appends a message to a game's log.
    pub async fn insert(
        conn: &mut SqliteConnection,
        game_id: i64,
        turn: i64,
        message: &str,
    ) -> Result<(), AppError> {
        sqlx::query("INSERT INTO event_log (game_id, turn, message) VALUES (?, ?, ?)")
            .bind(game_id)
            .bind(turn)
            .bind(message)
            .execute(conn)
            .await?;
        Ok(())
    }

    /// Returns a game's latest messages, newest first.
    pub async fn latest(
        conn: &mut SqliteConnection,
        game_id: i64,
        limit: i64,
    ) -> Result<Vec<EventRecord>, AppError> {
        let events = sqlx::query_as(
            "SELECT turn, message FROM event_log WHERE game_id = ? ORDER BY id DESC LIMIT ?",
        )
        .bind(game_id)
        .bind(limit)
        .fetch_all(conn)
        .await?;
        Ok(events)
    }
}
