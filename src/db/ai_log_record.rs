use sqlx::{FromRow, SqliteConnection};

use crate::domain::Phase;
use crate::error::AppError;

/// An entry of a game's AI log: one AI decision with its reasons, or a summary of the
/// table's hidden state. For developers only, never shown to players.
#[derive(Debug, Clone, FromRow)]
pub struct AiLogRecord {
    /// Turn of the entry.
    pub turn: i64,
    /// Database value of the phase the entry was written in.
    pub phase: String,
    /// Name of the alchemist who decided, `None` for an entry about the whole table.
    pub player_name: Option<String>,
    /// Text of the entry, possibly over several lines.
    pub body: String,
    /// When the entry was written, in UTC.
    pub created_at: String,
}

impl AiLogRecord {
    /// Appends an entry to a game's AI log.
    ///
    /// # Arguments
    /// * `player_id` - The deciding alchemist, or `None` for the whole table.
    ///
    /// # Errors
    /// A database error.
    pub async fn insert(
        conn: &mut SqliteConnection,
        game_id: i64,
        turn: i64,
        phase: Phase,
        player_id: Option<i64>,
        body: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO ai_log (game_id, turn, phase, player_id, body) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(game_id)
        .bind(turn)
        .bind(phase.as_db_str())
        .bind(player_id)
        .bind(body)
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Returns a game's whole AI log, oldest first.
    ///
    /// # Errors
    /// A database error.
    pub async fn list_for_game(
        conn: &mut SqliteConnection,
        game_id: i64,
    ) -> Result<Vec<AiLogRecord>, AppError> {
        let records = sqlx::query_as(
            "SELECT ai_log.turn, ai_log.phase, players.name AS player_name, ai_log.body,
                ai_log.created_at
             FROM ai_log LEFT JOIN players ON players.id = ai_log.player_id
             WHERE ai_log.game_id = ? ORDER BY ai_log.id",
        )
        .bind(game_id)
        .fetch_all(conn)
        .await?;
        Ok(records)
    }
}
