//! Use cases: each player action runs in one write transaction and bumps the game version.

mod ai_players;
mod announce_fanatic;
mod board_loader;
mod board_view;
mod city_quota_row;
mod city_sales;
mod city_view;
mod convert_site;
mod create_game;
mod end_turn;
mod game_view;
mod move_pawns;
mod move_rebels;
mod pawn_fight;
mod pawn_rules;
mod pay_expense;
mod peddler_turns;
mod player_action;
mod produce_items;
mod purchases;
mod rebel_fights;
mod resolve_fanatics;
mod seat_cycle;
mod sell_items;
mod trade_view;
mod write_transaction;

pub use announce_fanatic::announce_fanatic;
pub use board_view::{
    BoardView, ConversionOption, DestinationOption, PathView, RebelChoice, SiteView,
};
pub use city_quota_row::CityQuotaRow;
pub use city_sales::sell_in_city;
pub use city_view::{CityView, DistrictView, HallToken};
pub use convert_site::convert_site;
pub use create_game::{CreatedGame, create_game};
pub use game_view::{GameView, PlayerSummary};
pub use move_pawns::move_pawns;
pub use move_rebels::{RebelMove, move_rebels};
pub use pay_expense::pay_expense;
pub use produce_items::{produce_items, required_items};
pub use purchases::buy_from_bank;
pub use rebel_fights::attack_pawn;
pub use sell_items::sell_items;
pub use trade_view::{
    CargoOption, FightTarget, FightView, MoveChoice, PawnToken, PawnView, PurchaseView, SaleChoice,
    StopOption, TradeView,
};
