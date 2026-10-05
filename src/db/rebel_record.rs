use sqlx::{FromRow, SqliteConnection};

use crate::domain::SiteId;
use crate::error::AppError;

#[derive(FromRow)]
struct RebelRow {
    id: i64,
    site: i64,
}

/// A rebels pawn on the map, as stored in the `rebels` table.
#[derive(Debug, Clone, Copy)]
pub struct RebelRecord {
    /// Primary key.
    pub id: i64,
    /// Where the pawn stands.
    pub site: SiteId,
}

impl TryFrom<RebelRow> for RebelRecord {
    type Error = AppError;

    fn try_from(row: RebelRow) -> Result<RebelRecord, AppError> {
        let site = SiteId::from_db(row.site).ok_or_else(|| AppError::CorruptData {
            column: "rebels.site",
            value: row.site.to_string(),
        })?;
        Ok(RebelRecord { id: row.id, site })
    }
}

impl RebelRecord {
    /// Puts a new rebels pawn on a site.
    pub async fn insert(
        conn: &mut SqliteConnection,
        game_id: i64,
        site: SiteId,
    ) -> Result<(), AppError> {
        sqlx::query("INSERT INTO rebels (game_id, site) VALUES (?, ?)")
            .bind(game_id)
            .bind(site.as_db())
            .execute(conn)
            .await?;
        Ok(())
    }

    /// Lists the rebels pawns of a game's map, oldest first.
    pub async fn list_for_game(
        conn: &mut SqliteConnection,
        game_id: i64,
    ) -> Result<Vec<RebelRecord>, AppError> {
        let rows: Vec<RebelRow> =
            sqlx::query_as("SELECT id, site FROM rebels WHERE game_id = ? ORDER BY id")
                .bind(game_id)
                .fetch_all(conn)
                .await?;
        rows.into_iter().map(RebelRecord::try_from).collect()
    }

    /// Moves a pawn to another site.
    pub async fn move_to(
        conn: &mut SqliteConnection,
        id: i64,
        site: SiteId,
    ) -> Result<(), AppError> {
        sqlx::query("UPDATE rebels SET site = ? WHERE id = ?")
            .bind(site.as_db())
            .bind(id)
            .execute(conn)
            .await?;
        Ok(())
    }
}
