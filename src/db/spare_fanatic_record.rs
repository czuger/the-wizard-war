use sqlx::{FromRow, SqliteConnection};

use crate::domain::{Cult, Fanatic};
use crate::error::AppError;

#[derive(FromRow)]
struct SpareFanaticRow {
    cult: String,
    strength: i64,
}

/// A fanatic pawn dealt to nobody, as stored in the `spare_fanatics` table.
#[derive(Debug, Clone)]
pub struct SpareFanaticRecord;

impl SpareFanaticRecord {
    /// Leaves a fanatic pawn in the box of a game.
    pub async fn insert(
        conn: &mut SqliteConnection,
        game_id: i64,
        fanatic: Fanatic,
    ) -> Result<(), AppError> {
        sqlx::query("INSERT INTO spare_fanatics (game_id, cult, strength) VALUES (?, ?, ?)")
            .bind(game_id)
            .bind(fanatic.cult.as_db_str())
            .bind(fanatic.strength)
            .execute(conn)
            .await?;
        Ok(())
    }

    /// Lists the pawns left in the box of a game.
    pub async fn list_for_game(
        conn: &mut SqliteConnection,
        game_id: i64,
    ) -> Result<Vec<Fanatic>, AppError> {
        let rows: Vec<SpareFanaticRow> = sqlx::query_as(
            "SELECT cult, strength FROM spare_fanatics WHERE game_id = ? ORDER BY id",
        )
        .bind(game_id)
        .fetch_all(conn)
        .await?;
        rows.into_iter()
            .map(|row| {
                let cult = Cult::from_db_str(&row.cult).ok_or_else(|| AppError::CorruptData {
                    column: "spare_fanatics.cult",
                    value: row.cult.clone(),
                })?;
                Ok(Fanatic::new(cult, row.strength))
            })
            .collect()
    }
}
