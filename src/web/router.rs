use std::path::Path;

use axum::Router;
use axum::http::HeaderValue;
use axum::http::header::CACHE_CONTROL;
use axum::routing::{get, post};
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;

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
            &format!("{player_path}/conversion"),
            post(action_handlers::convert),
        )
        .route(
            &format!("{player_path}/rebels"),
            post(action_handlers::move_rebel_pawns),
        )
        .route(
            &format!("{player_path}/fight"),
            post(action_handlers::fight),
        )
        .route(
            &format!("{player_path}/production"),
            post(action_handlers::produce),
        )
        .route(
            &format!("{player_path}/pawns"),
            post(action_handlers::move_pawn_tokens),
        )
        .route(&format!("{player_path}/sales"), post(action_handlers::sell))
        .route(
            &format!("{player_path}/transfer"),
            post(action_handlers::transfer),
        )
        .route(
            &format!("{player_path}/city-sale"),
            post(action_handlers::sell_city),
        )
        .route(
            &format!("{player_path}/purchase"),
            post(action_handlers::purchase),
        )
        .nest_service("/img", ServeDir::new(asset_root.join("img")))
        .nest_service("/static", ServeDir::new(asset_root.join("static")))
        // Assets keep stable URLs, so browsers must revalidate them (a 304 when unchanged) to see edits.
        .layer(SetResponseHeaderLayer::if_not_present(
            CACHE_CONTROL,
            HeaderValue::from_static("no-cache"),
        ))
        .with_state(state)
}
