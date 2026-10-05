use sqlx::{FromRow, SqliteConnection};

use crate::domain::rules::{STARTING_LABORATORIES, STARTING_MONEY};
use crate::domain::{District, ItemCounts};
use crate::error::AppError;

const PLAYER_COLUMNS: &str = "id, game_id, seat, name, token, money, laboratories,
    toratsa_in_production, xephis_in_production, yaboul_in_production,
    toratsa_in_stock, xephis_in_stock, yaboul_in_stock, turn_expense, announcement_done, expense_done, is_ai,
    district, moves_done";

#[derive(FromRow)]
struct PlayerRow {
    id: i64,
    game_id: i64,
    seat: i64,
    name: String,
    token: String,
    money: i64,
    laboratories: i64,
    toratsa_in_production: i64,
    xephis_in_production: i64,
    yaboul_in_production: i64,
    toratsa_in_stock: i64,
    xephis_in_stock: i64,
    yaboul_in_stock: i64,
    turn_expense: i64,
    announcement_done: bool,
    expense_done: bool,
    is_ai: bool,
    district: String,
    moves_done: bool,
}

/// An alchemist as stored in the `players` table.
#[derive(Debug, Clone)]
pub struct PlayerRecord {
    /// Primary key.
    pub id: i64,
    /// Game the player belongs to.
    pub game_id: i64,
    /// Position around the table, from 0.
    pub seat: i64,
    /// Name shown to everyone.
    pub name: String,
    /// Secret part of the player's link.
    pub token: String,
    /// Écus owned.
    pub money: i64,
    /// Laboratories owned.
    pub laboratories: i64,
    /// Items being made this turn.
    pub production: ItemCounts,
    /// Items already made.
    pub stock: ItemCounts,
    /// Écus paid into the collective expense this turn.
    pub turn_expense: i64,
    /// Whether the player has finished this turn's announcement.
    pub announcement_done: bool,
    /// Whether the player has paid this turn's expense.
    pub expense_done: bool,
    /// Whether the computer plays this alchemist.
    pub is_ai: bool,
    /// District of the city holding the player's laboratories; their pawns share its colour.
    pub district: District,
    /// Whether the player has moved their pawns in this turn's phase V.
    pub moves_done: bool,
}

impl TryFrom<PlayerRow> for PlayerRecord {
    type Error = AppError;

    fn try_from(row: PlayerRow) -> Result<PlayerRecord, AppError> {
        let district = District::from_key(&row.district).ok_or_else(|| AppError::CorruptData {
            column: "players.district",
            value: row.district.clone(),
        })?;
        Ok(PlayerRecord {
            id: row.id,
            game_id: row.game_id,
            seat: row.seat,
            name: row.name,
            token: row.token,
            money: row.money,
            laboratories: row.laboratories,
            production: ItemCounts::new(
                row.toratsa_in_production,
                row.xephis_in_production,
                row.yaboul_in_production,
            ),
            stock: ItemCounts::new(
                row.toratsa_in_stock,
                row.xephis_in_stock,
                row.yaboul_in_stock,
            ),
            turn_expense: row.turn_expense,
            announcement_done: row.announcement_done,
            expense_done: row.expense_done,
            is_ai: row.is_ai,
            district,
            moves_done: row.moves_done,
        })
    }
}

impl PlayerRecord {
    /// Inserts a new player with the starting money and laboratories.
    ///
    /// # Arguments
    /// * `is_ai` - whether the computer plays this alchemist.
    /// * `district` - district of the player's laboratories.
    ///
    /// # Returns
    /// The new player's id.
    pub async fn insert(
        conn: &mut SqliteConnection,
        game_id: i64,
        seat: i64,
        name: &str,
        token: &str,
        is_ai: bool,
        district: District,
    ) -> Result<i64, AppError> {
        let result = sqlx::query(
            "INSERT INTO players (game_id, seat, name, token, money, laboratories, is_ai, district)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(game_id)
        .bind(seat)
        .bind(name)
        .bind(token)
        .bind(STARTING_MONEY)
        .bind(STARTING_LABORATORIES)
        .bind(is_ai)
        .bind(district.key())
        .execute(conn)
        .await?;
        Ok(result.last_insert_rowid())
    }

    /// Finds the player of a game from the token in their link.
    ///
    /// # Errors
    /// [`AppError::PlayerNotFound`] when the token does not belong to this game.
    pub async fn find_by_token(
        conn: &mut SqliteConnection,
        game_id: i64,
        token: &str,
    ) -> Result<PlayerRecord, AppError> {
        let sql = format!("SELECT {PLAYER_COLUMNS} FROM players WHERE game_id = ? AND token = ?");
        // safe: only the constant column list is interpolated.
        let row: Option<PlayerRow> = sqlx::query_as(sqlx::AssertSqlSafe(sql))
            .bind(game_id)
            .bind(token)
            .fetch_optional(conn)
            .await?;
        row.ok_or(AppError::PlayerNotFound)?.try_into()
    }

    /// Lists the players of a game by seat.
    pub async fn list_for_game(
        conn: &mut SqliteConnection,
        game_id: i64,
    ) -> Result<Vec<PlayerRecord>, AppError> {
        let sql = format!("SELECT {PLAYER_COLUMNS} FROM players WHERE game_id = ? ORDER BY seat");
        // safe: only the constant column list is interpolated.
        let rows: Vec<PlayerRow> = sqlx::query_as(sqlx::AssertSqlSafe(sql))
            .bind(game_id)
            .fetch_all(conn)
            .await?;
        rows.into_iter().map(PlayerRecord::try_from).collect()
    }

    /// Writes every mutable field back.
    pub async fn save(&self, conn: &mut SqliteConnection) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE players SET money = ?, laboratories = ?,
                    toratsa_in_production = ?, xephis_in_production = ?, yaboul_in_production = ?,
                    toratsa_in_stock = ?, xephis_in_stock = ?, yaboul_in_stock = ?,
                    turn_expense = ?, announcement_done = ?, expense_done = ?, moves_done = ?
             WHERE id = ?",
        )
        .bind(self.money)
        .bind(self.laboratories)
        .bind(self.production.toratsa)
        .bind(self.production.xephis)
        .bind(self.production.yaboul)
        .bind(self.stock.toratsa)
        .bind(self.stock.xephis)
        .bind(self.stock.yaboul)
        .bind(self.turn_expense)
        .bind(self.announcement_done)
        .bind(self.expense_done)
        .bind(self.moves_done)
        .bind(self.id)
        .execute(conn)
        .await?;
        Ok(())
    }
}
