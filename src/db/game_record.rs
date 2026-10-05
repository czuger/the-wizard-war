use sqlx::{FromRow, SqliteConnection};

use crate::domain::{ItemCounts, Phase};
use crate::error::AppError;

#[derive(FromRow)]
struct GameRow {
    id: i64,
    phase: String,
    turn: i64,
    max_turns: i64,
    first_seat: i64,
    active_seat: Option<i64>,
    supply_toratsa: i64,
    supply_xephis: i64,
    supply_yaboul: i64,
    collective_expense: i64,
    city_sales_left: i64,
    city_passes: i64,
    version: i64,
}

/// A game as stored in the `games` table.
#[derive(Debug, Clone)]
pub struct GameRecord {
    /// Primary key.
    pub id: i64,
    /// Current phase of the turn.
    pub phase: Phase,
    /// Current turn, starting at 1.
    pub turn: i64,
    /// Turn after which the game ends.
    pub max_turns: i64,
    /// Seat of this turn's first player.
    pub first_seat: i64,
    /// Seat expected to act in a sequential phase.
    pub active_seat: Option<i64>,
    /// Magic items left in the bank's envelope.
    pub supply: ItemCounts,
    /// Écus paid into the collective expense this turn.
    pub collective_expense: i64,
    /// Items that may still be sold in the city this turn (phase VI).
    pub city_sales_left: i64,
    /// Players who passed in a row in phase VI; a full round of passes ends the sales.
    pub city_passes: i64,
    /// Incremented on every change, so pages know when to refresh.
    pub version: i64,
}

impl TryFrom<GameRow> for GameRecord {
    type Error = AppError;

    fn try_from(row: GameRow) -> Result<GameRecord, AppError> {
        let phase = Phase::from_db_str(&row.phase).ok_or_else(|| AppError::CorruptData {
            column: "games.phase",
            value: row.phase.clone(),
        })?;
        Ok(GameRecord {
            id: row.id,
            phase,
            turn: row.turn,
            max_turns: row.max_turns,
            first_seat: row.first_seat,
            active_seat: row.active_seat,
            supply: ItemCounts::new(row.supply_toratsa, row.supply_xephis, row.supply_yaboul),
            collective_expense: row.collective_expense,
            city_sales_left: row.city_sales_left,
            city_passes: row.city_passes,
            version: row.version,
        })
    }
}

impl GameRecord {
    /// Inserts a new game at turn 1, in the announcements phase.
    ///
    /// # Returns
    /// The new game's id.
    pub async fn insert(
        conn: &mut SqliteConnection,
        max_turns: i64,
        first_seat: i64,
        supply: ItemCounts,
    ) -> Result<i64, AppError> {
        let result = sqlx::query(
            "INSERT INTO games (phase, turn, max_turns, first_seat, supply_toratsa, supply_xephis, supply_yaboul)
             VALUES (?, 1, ?, ?, ?, ?, ?)",
        )
        .bind(Phase::Announcements.as_db_str())
        .bind(max_turns)
        .bind(first_seat)
        .bind(supply.toratsa)
        .bind(supply.xephis)
        .bind(supply.yaboul)
        .execute(conn)
        .await?;
        Ok(result.last_insert_rowid())
    }

    /// Loads a game.
    ///
    /// # Errors
    /// [`AppError::GameNotFound`] when no game has this id.
    pub async fn find(conn: &mut SqliteConnection, id: i64) -> Result<GameRecord, AppError> {
        let row: Option<GameRow> = sqlx::query_as(
            "SELECT id, phase, turn, max_turns, first_seat, active_seat, supply_toratsa, supply_xephis,
                    supply_yaboul, collective_expense, city_sales_left, city_passes, version
             FROM games WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(conn)
        .await?;
        row.ok_or(AppError::GameNotFound)?.try_into()
    }

    /// Returns the game's change counter.
    ///
    /// # Errors
    /// [`AppError::GameNotFound`] when no game has this id.
    pub async fn version_of(conn: &mut SqliteConnection, id: i64) -> Result<i64, AppError> {
        let version: Option<i64> = sqlx::query_scalar("SELECT version FROM games WHERE id = ?")
            .bind(id)
            .fetch_optional(conn)
            .await?;
        version.ok_or(AppError::GameNotFound)
    }

    /// Writes every mutable field back and increments the version.
    pub async fn save(&self, conn: &mut SqliteConnection) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE games SET phase = ?, turn = ?, first_seat = ?, active_seat = ?, supply_toratsa = ?,
                    supply_xephis = ?, supply_yaboul = ?, collective_expense = ?, city_sales_left = ?, city_passes = ?,
                    version = version + 1
             WHERE id = ?",
        )
        .bind(self.phase.as_db_str())
        .bind(self.turn)
        .bind(self.first_seat)
        .bind(self.active_seat)
        .bind(self.supply.toratsa)
        .bind(self.supply.xephis)
        .bind(self.supply.yaboul)
        .bind(self.collective_expense)
        .bind(self.city_sales_left)
        .bind(self.city_passes)
        .bind(self.id)
        .execute(conn)
        .await?;
        Ok(())
    }
}
