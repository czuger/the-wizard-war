//! Game rules, free of storage and web concerns.

pub mod ai_strategy;
pub mod board;
mod board_state;
mod conversion;
mod cult;
mod cult_marker;
mod fanatic;
mod fanatic_pool;
mod fanatic_resolution;
mod item_counts;
mod magic_item;
mod phase;
pub mod rebel_movement;
pub mod rules;
mod site;
mod site_kind;
mod turn_order;
mod zone;

pub use board_state::BoardState;
pub use conversion::{Conversion, ConversionError};
pub use cult::Cult;
pub use cult_marker::CultMarker;
pub use fanatic::Fanatic;
pub use fanatic_pool::FanaticPool;
pub use fanatic_resolution::{FanaticVerdict, dominant_cult};
pub use item_counts::ItemCounts;
pub use magic_item::MagicItem;
pub use phase::Phase;
pub use site::{Site, SiteId};
pub use site_kind::SiteKind;
pub use turn_order::TurnOrder;
pub use zone::Zone;
