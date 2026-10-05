use sqlx::{FromRow, SqliteConnection};

use crate::domain::{Cult, Fanatic};
use crate::error::AppError;

#[derive(FromRow)]
struct FanaticRow {
    id: i64,
    player_id: i64,
    cult: String,
    strength: i64,
    in_hall: bool,
    revealed: bool,
}

/// A fanatic pawn as stored in the `fanatics` table.
#[derive(Debug, Clone)]
pub struct FanaticRecord {
    /// Primary key.
    pub id: i64,
    /// Player who owns the pawn.
    pub player_id: i64,
    /// The pawn itself.
    pub fanatic: Fanatic,
    /// Whether the pawn waits face down in the great hall.
    pub in_hall: bool,
    /// Whether the pawn lies face up in the hall, after phase IV and until the end of the turn.
    pub revealed: bool,
}

impl TryFrom<FanaticRow> for FanaticRecord {
    type Error = AppError;

    fn try_from(row: FanaticRow) -> Result<FanaticRecord, AppError> {
        let cult = Cult::from_db_str(&row.cult).ok_or_else(|| AppError::CorruptData {
            column: "fanatics.cult",
            value: row.cult.clone(),
        })?;
        Ok(FanaticRecord {
            id: row.id,
            player_id: row.player_id,
            fanatic: Fanatic::new(cult, row.strength),
            in_hall: row.in_hall,
            revealed: row.revealed,
        })
    }
}

impl FanaticRecord {
    /// Gives a fanatic pawn to a player.
    pub async fn insert(
        conn: &mut SqliteConnection,
        game_id: i64,
        player_id: i64,
        fanatic: Fanatic,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO fanatics (game_id, player_id, cult, strength) VALUES (?, ?, ?, ?)",
        )
        .bind(game_id)
        .bind(player_id)
        .bind(fanatic.cult.as_db_str())
        .bind(fanatic.strength)
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Lists a player's pawns, in the hall or not, strongest Xéphis first.
    pub async fn list_for_player(
        conn: &mut SqliteConnection,
        player_id: i64,
    ) -> Result<Vec<FanaticRecord>, AppError> {
        let rows: Vec<FanaticRow> = sqlx::query_as(
            "SELECT id, player_id, cult, strength, in_hall, revealed FROM fanatics
             WHERE player_id = ? ORDER BY cult, strength DESC, id",
        )
        .bind(player_id)
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(FanaticRecord::try_from).collect()
    }

    /// Lists the pawns waiting in a game's great hall.
    pub async fn list_in_hall(
        conn: &mut SqliteConnection,
        game_id: i64,
    ) -> Result<Vec<FanaticRecord>, AppError> {
        let rows: Vec<FanaticRow> = sqlx::query_as(
            "SELECT id, player_id, cult, strength, in_hall, revealed FROM fanatics
             WHERE game_id = ? AND in_hall = 1 ORDER BY id",
        )
        .bind(game_id)
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(FanaticRecord::try_from).collect()
    }

    /// Sends a pawn to the great hall.
    pub async fn move_to_hall(conn: &mut SqliteConnection, id: i64) -> Result<(), AppError> {
        sqlx::query("UPDATE fanatics SET in_hall = 1 WHERE id = ?")
            .bind(id)
            .execute(conn)
            .await?;
        Ok(())
    }

    /// Turns every pawn of the great hall face up (phase IV).
    pub async fn reveal_hall(conn: &mut SqliteConnection, game_id: i64) -> Result<(), AppError> {
        sqlx::query("UPDATE fanatics SET revealed = 1 WHERE game_id = ? AND in_hall = 1")
            .bind(game_id)
            .execute(conn)
            .await?;
        Ok(())
    }

    /// Removes from the game every pawn that went through the great hall.
    pub async fn discard_hall(conn: &mut SqliteConnection, game_id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM fanatics WHERE game_id = ? AND in_hall = 1")
            .bind(game_id)
            .execute(conn)
            .await?;
        Ok(())
    }
}
