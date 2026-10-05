use sqlx::SqliteConnection;

use crate::db::{CultMarkerRecord, RebelRecord};
use crate::domain::BoardState;
use crate::error::AppError;

/// Loads the markers and rebels of a game's map.
pub(crate) async fn load_board_state(
    conn: &mut SqliteConnection,
    game_id: i64,
) -> Result<BoardState, AppError> {
    let markers = CultMarkerRecord::list_for_game(conn, game_id)
        .await?
        .into_iter()
        .map(|record| (record.site, record.marker))
        .collect();
    let rebels = RebelRecord::list_for_game(conn, game_id)
        .await?
        .into_iter()
        .map(|record| record.site)
        .collect();
    Ok(BoardState::new(markers, rebels))
}
