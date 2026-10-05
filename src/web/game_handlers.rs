use askama::Template;
use axum::extract::{Path, State};
use axum::response::Html;

use crate::db::GameRecord;
use crate::error::AppError;
use crate::service::GameView;
use crate::web::app_state::AppState;
use crate::web::game_page::GamePage;

/// `GET /games/{game_id}/players/{token}`: the player's page.
pub async fn show_game(
    State(state): State<AppState>,
    Path((game_id, token)): Path<(i64, String)>,
) -> Result<Html<String>, AppError> {
    let view = GameView::load(&state.pool, game_id, &token).await?;
    Ok(Html(GamePage::new(view, None).render()?))
}

/// `GET /games/{game_id}/version`: the game's change counter, polled by open pages.
pub async fn show_version(
    State(state): State<AppState>,
    Path(game_id): Path<i64>,
) -> Result<String, AppError> {
    let mut conn = state.pool.acquire().await?;
    Ok(GameRecord::version_of(&mut conn, game_id)
        .await?
        .to_string())
}
