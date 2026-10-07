//! SQLite storage: one record type per table.

mod cult_marker_record;
mod database;
mod event_record;
mod fanatic_record;
mod game_record;
mod pawn_record;
mod player_record;
mod rebel_record;
mod sale_record;
mod spare_fanatic_record;

pub use cult_marker_record::CultMarkerRecord;
pub use database::connect;
pub use event_record::EventRecord;
pub use fanatic_record::FanaticRecord;
pub use game_record::GameRecord;
pub use pawn_record::PawnRecord;
pub use player_record::{NewPlayer, PlayerRecord};
pub use rebel_record::RebelRecord;
pub use sale_record::SaleRecord;
pub use spare_fanatic_record::SpareFanaticRecord;
