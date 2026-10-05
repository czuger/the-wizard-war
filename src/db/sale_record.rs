use sqlx::SqliteConnection;

use crate::domain::SiteId;
use crate::error::AppError;

/// Items sold on the map, as stored in the `sales` table: one row per item sold.
pub struct SaleRecord;

impl SaleRecord {
    /// Records that an item was sold on `site` during `turn`.
    pub async fn insert(
        conn: &mut SqliteConnection,
        game_id: i64,
        turn: i64,
        site: SiteId,
    ) -> Result<(), AppError> {
        sqlx::query("INSERT INTO sales (game_id, turn, site) VALUES (?, ?, ?)")
            .bind(game_id)
            .bind(turn)
            .bind(site.as_db())
            .execute(conn)
            .await?;
        Ok(())
    }

    /// Lists the sites where an item was sold during `turn`.
    pub async fn sites_sold(
        conn: &mut SqliteConnection,
        game_id: i64,
        turn: i64,
    ) -> Result<Vec<SiteId>, AppError> {
        let sites: Vec<i64> =
            sqlx::query_scalar("SELECT DISTINCT site FROM sales WHERE game_id = ? AND turn = ?")
                .bind(game_id)
                .bind(turn)
                .fetch_all(conn)
                .await?;
        Ok(sites.into_iter().filter_map(SiteId::from_db).collect())
    }
}
