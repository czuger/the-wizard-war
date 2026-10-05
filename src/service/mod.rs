//! Use cases: each player action runs in one write transaction and bumps the game version.

mod ai_players;
mod announce_fanatic;
mod create_game;
mod end_turn;
mod game_view;
mod pay_expense;
mod player_action;
mod produce_items;
mod resolve_fanatics;
mod write_transaction;

pub use announce_fanatic::announce_fanatic;
pub use create_game::{CreatedGame, create_game};
pub use game_view::{GameView, PlayerSummary};
pub use pay_expense::pay_expense;
pub use produce_items::{produce_items, required_items};
