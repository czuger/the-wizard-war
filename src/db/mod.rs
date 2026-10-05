//! SQLite storage: one record type per table.

mod database;
mod event_record;
mod fanatic_record;
mod game_record;
mod player_record;

pub use database::connect;
pub use event_record::EventRecord;
pub use fanatic_record::FanaticRecord;
pub use game_record::GameRecord;
pub use player_record::PlayerRecord;
