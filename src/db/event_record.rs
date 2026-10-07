use sqlx::{FromRow, SqliteConnection};

use crate::domain::LogSection;
use crate::error::AppError;

#[derive(FromRow)]
struct EventRow {
    turn: i64,
    section: Option<String>,
    message: String,
}

/// A line of the game log, shown to every player.
#[derive(Debug, Clone)]
pub struct EventRecord {
    /// Turn during which the event happened.
    pub turn: i64,
    /// Part of the turn the line belongs to, `None` for lines logged before it was recorded.
    pub section: Option<LogSection>,
    /// French text shown to players.
    pub message: String,
}

impl EventRecord {
    /// Appends a message to a game's log.
    pub async fn insert(
        conn: &mut SqliteConnection,
        game_id: i64,
        turn: i64,
        section: LogSection,
        message: &str,
    ) -> Result<(), AppError> {
        sqlx::query("INSERT INTO event_log (game_id, turn, section, message) VALUES (?, ?, ?, ?)")
            .bind(game_id)
            .bind(turn)
            .bind(section.as_db_str())
            .bind(message)
            .execute(conn)
            .await?;
        Ok(())
    }

    /// Returns a game's whole log, newest first.
    ///
    /// # Errors
    /// [`AppError::CorruptData`] for an unknown section, or a database error.
    pub async fn list_for_game(
        conn: &mut SqliteConnection,
        game_id: i64,
    ) -> Result<Vec<EventRecord>, AppError> {
        let rows: Vec<EventRow> = sqlx::query_as(
            "SELECT turn, section, message FROM event_log WHERE game_id = ? ORDER BY id DESC",
        )
        .bind(game_id)
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(EventRecord::from_row).collect()
    }

    fn from_row(row: EventRow) -> Result<EventRecord, AppError> {
        let section = row
            .section
            .map(|value| {
                LogSection::from_db_str(&value).ok_or(AppError::CorruptData {
                    column: "event_log.section",
                    value,
                })
            })
            .transpose()?;
        Ok(EventRecord {
            turn: row.turn,
            section,
            message: row.message,
        })
    }
}
