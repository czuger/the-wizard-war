use crate::domain::{Location, MagicItem};

/// Where a player sends one of their peddlers or caravans in phase V.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PawnOrder {
    /// The pawn moved.
    pub pawn_id: i64,
    /// Where it ends its move.
    pub to: Location,
    /// The item from stock it loads when it leaves the city; `None` for a pawn on the map.
    pub item: Option<MagicItem>,
}
