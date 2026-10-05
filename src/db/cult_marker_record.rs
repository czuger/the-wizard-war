use sqlx::{FromRow, SqliteConnection};

use crate::domain::{Cult, CultMarker, SiteId};
use crate::error::AppError;

#[derive(FromRow)]
struct CultMarkerRow {
    site: i64,
    cult: String,
    face_up: bool,
}

/// A cult marker on the map, as stored in the `cult_markers` table.
#[derive(Debug, Clone, Copy)]
pub struct CultMarkerRecord {
    /// The site holding the marker.
    pub site: SiteId,
    /// The marker itself.
    pub marker: CultMarker,
}

impl TryFrom<CultMarkerRow> for CultMarkerRecord {
    type Error = AppError;

    fn try_from(row: CultMarkerRow) -> Result<CultMarkerRecord, AppError> {
        let site = SiteId::from_db(row.site).ok_or_else(|| AppError::CorruptData {
            column: "cult_markers.site",
            value: row.site.to_string(),
        })?;
        let cult = Cult::from_db_str(&row.cult).ok_or_else(|| AppError::CorruptData {
            column: "cult_markers.cult",
            value: row.cult.clone(),
        })?;
        Ok(CultMarkerRecord {
            site,
            marker: CultMarker {
                cult,
                face_up: row.face_up,
            },
        })
    }
}

impl CultMarkerRecord {
    /// Lists the markers of a game's map, by site.
    pub async fn list_for_game(
        conn: &mut SqliteConnection,
        game_id: i64,
    ) -> Result<Vec<CultMarkerRecord>, AppError> {
        let rows: Vec<CultMarkerRow> = sqlx::query_as(
            "SELECT site, cult, face_up FROM cult_markers WHERE game_id = ? ORDER BY site",
        )
        .bind(game_id)
        .fetch_all(conn)
        .await?;
        rows.into_iter().map(CultMarkerRecord::try_from).collect()
    }

    /// Puts this marker on its site, replacing the one already there.
    pub async fn save(&self, conn: &mut SqliteConnection, game_id: i64) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO cult_markers (game_id, site, cult, face_up) VALUES (?, ?, ?, ?)
             ON CONFLICT (game_id, site) DO UPDATE SET cult = excluded.cult, face_up = excluded.face_up",
        )
        .bind(game_id)
        .bind(self.site.as_db())
        .bind(self.marker.cult.as_db_str())
        .bind(self.marker.face_up)
        .execute(conn)
        .await?;
        Ok(())
    }
}
