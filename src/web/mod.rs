//! HTTP layer: routes, handlers and page templates.

mod action_handlers;
mod app_state;
mod error_page;
mod game_handlers;
mod game_page;
mod lobby_handlers;
mod router;

pub use app_state::AppState;
pub use router::build_router;
