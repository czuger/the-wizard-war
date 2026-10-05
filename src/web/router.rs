use std::path::Path;

use axum::Router;
use axum::routing::{get, post};
use tower_http::services::ServeDir;

use crate::web::app_state::AppState;
use crate::web::{action_handlers, game_handlers, lobby_handlers};

/// Builds every route of the application.
///
/// # Arguments
/// * `asset_root` - directory holding the `img/` and `static/` folders.
pub fn build_router(state: AppState, asset_root: &Path) -> Router {
    let player_path = "/games/{game_id}/players/{token}";
    Router::new()
        .route("/", get(lobby_handlers::show_lobby))
        .route("/games", post(lobby_handlers::create_game))
        .route("/games/{game_id}/version", get(game_handlers::show_version))
        .route(player_path, get(game_handlers::show_game))
        .route(
            &format!("{player_path}/announcement"),
            post(action_handlers::announce),
        )
        .route(
            &format!("{player_path}/expense"),
            post(action_handlers::pay),
        )
        .route(
            &format!("{player_path}/production"),
            post(action_handlers::produce),
        )
        .nest_service("/img", ServeDir::new(asset_root.join("img")))
        .nest_service("/static", ServeDir::new(asset_root.join("static")))
        .with_state(state)
}
