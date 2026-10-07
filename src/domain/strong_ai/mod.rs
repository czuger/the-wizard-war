//! Decisions of the « Fort » computer-controlled alchemists.
//!
//! Each decision scores the legal options by what they are expected to be worth, in écus,
//! by the end of the game, from what the alchemist could know at the table: its own hand,
//! stock, pawns and money, the supply, the map with every pawn (but not the rivals' loads),
//! the number of fanatics in the hall, the fanatics revealed so far, how rich the rivals
//! look, and the turns left (see [`AiOutlook`]).

mod announcement;
mod buyer;
mod city_sale;
mod conversion;
mod expense;
mod fanatic_odds;
mod fight_target;
mod item_values;
mod near_best;
mod outlook;
mod pawn_moves;
mod production;
mod purchase;
mod rebel_moves;

pub use announcement::choose_announcement;
pub use buyer::{Buyer, FLIP_RISK_PER_TURN, best_buyer, buyers, travel_turns};
pub use city_sale::choose_city_sale;
pub use conversion::choose_conversion;
pub use expense::choose_expense;
pub use fanatic_odds::FanaticOdds;
pub use fight_target::{FightCandidate, choose_fight_target};
pub use item_values::ItemValues;
pub use near_best::pick_near_best;
pub use outlook::{AiOutlook, RivalPawn};
pub use pawn_moves::choose_pawn_moves;
pub use production::choose_production;
pub use purchase::choose_purchase;
pub use rebel_moves::{Rebel, choose_rebel_moves};
