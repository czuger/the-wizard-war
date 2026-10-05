use sqlx::{FromRow, SqliteConnection};

use crate::domain::{Location, MagicItem, PawnKind};
use crate::error::AppError;

#[derive(FromRow)]
struct PawnRow {
    id: i64,
    player_id: i64,
    kind: String,
    site: Option<i64>,
    item: Option<String>,
    stopped_until_turn: i64,
}

/// A peddler or caravan, as stored in the `pawns` table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PawnRecord {
    /// Primary key.
    pub id: i64,
    /// Player who owns the pawn.
    pub player_id: i64,
    /// Peddler or caravan.
    pub kind: PawnKind,
    /// In the city, or on a site of the map.
    pub location: Location,
    /// The item it carries, face down, if any.
    pub item: Option<MagicItem>,
    /// Last turn in which the pawn, turned over after a fight, may not move.
    pub stopped_until_turn: i64,
}

impl TryFrom<PawnRow> for PawnRecord {
    type Error = AppError;

    fn try_from(row: PawnRow) -> Result<PawnRecord, AppError> {
        let corrupt = |column: &'static str, value: String| AppError::CorruptData { column, value };
        let kind =
            PawnKind::from_key(&row.kind).ok_or_else(|| corrupt("pawns.kind", row.kind.clone()))?;
        let location = Location::from_db(row.site)
            .ok_or_else(|| corrupt("pawns.site", format!("{:?}", row.site)))?;
        let item = match row.item {
            None => None,
            Some(key) => {
                Some(MagicItem::from_key(&key).ok_or_else(|| corrupt("pawns.item", key.clone()))?)
            }
        };
        Ok(PawnRecord {
            id: row.id,
            player_id: row.player_id,
            kind,
            location,
            item,
            stopped_until_turn: row.stopped_until_turn,
        })
    }
}

impl PawnRecord {
    /// Gives a new, empty pawn to a player, in the city.
    pub async fn insert(
        conn: &mut SqliteConnection,
        game_id: i64,
        player_id: i64,
        kind: PawnKind,
    ) -> Result<(), AppError> {
        sqlx::query("INSERT INTO pawns (game_id, player_id, kind) VALUES (?, ?, ?)")
            .bind(game_id)
            .bind(player_id)
            .bind(kind.key())
            .execute(conn)
            .await?;
        Ok(())
    }

    /// Lists the pawns of a game, oldest first.
    pub async fn list_for_game(
        conn: &mut SqliteConnection,
        game_id: i64,
    ) -> Result<Vec<PawnRecord>, AppError> {
        let rows: Vec<PawnRow> = sqlx::query_as(
            "SELECT id, player_id, kind, site, item, stopped_until_turn
             FROM pawns WHERE game_id = ? ORDER BY id",
        )
        .bind(game_id)
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(PawnRecord::try_from).collect()
    }

    /// Writes the location, item and stop back.
    pub async fn save(&self, conn: &mut SqliteConnection) -> Result<(), AppError> {
        sqlx::query("UPDATE pawns SET site = ?, item = ?, stopped_until_turn = ? WHERE id = ?")
            .bind(self.location.as_db())
            .bind(self.item.map(MagicItem::key))
            .bind(self.stopped_until_turn)
            .bind(self.id)
            .execute(conn)
            .await?;
        Ok(())
    }

    /// Removes a pawn from the game.
    pub async fn delete(conn: &mut SqliteConnection, id: i64) -> Result<(), AppError> {
        sqlx::query("DELETE FROM pawns WHERE id = ?")
            .bind(id)
            .execute(conn)
            .await?;
        Ok(())
    }

    /// Whether the pawn may move in phase V of `turn`.
    pub fn can_move(&self, turn: i64) -> bool {
        self.stopped_until_turn < turn
    }
}
