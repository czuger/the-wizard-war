//! What a strong AI knows when it decides: only what its alchemist could see at the table.

use crate::domain::ai_trading::MovablePawn;
use crate::domain::strong_ai::item_values::ItemValues;
use crate::domain::{BoardState, Fanatic, ItemCounts, Location, PawnKind, SiteId, Wealth};

/// A rival's peddler or caravan, as everyone sees it: its load is hidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RivalPawn {
    /// Where it stands.
    pub location: Location,
    /// Peddler or caravan.
    pub kind: PawnKind,
    /// How rich its owner looks.
    pub owner_wealth: Wealth,
}

/// The public facts and the AI's own hidden ones, gathered for one decision.
#[derive(Debug, Clone, PartialEq)]
pub struct AiOutlook {
    /// Current turn, from 1.
    pub turn: i64,
    /// Turn after which the game ends.
    pub max_turns: i64,
    /// Phases V left in the game, this turn's included if it is still ahead.
    pub sale_turns: i64,
    /// Markers and rebels on the map.
    pub board: BoardState,
    /// The AI's écus.
    pub money: i64,
    /// The AI's laboratories.
    pub laboratories: i64,
    /// Laboratories of every player.
    pub total_laboratories: i64,
    /// The AI's stock.
    pub stock: ItemCounts,
    /// Items left in the supply.
    pub supply: ItemCounts,
    /// The AI's fanatics not in the hall.
    pub hand: Vec<Fanatic>,
    /// The AI's fanatic in the hall this turn, if any.
    pub own_in_hall: Option<Fanatic>,
    /// Fanatics the other players put face down in the hall this turn.
    pub others_in_hall: usize,
    /// Fanatics the AI has not seen: the full pool minus its own and the revealed ones.
    pub unseen: Vec<Fanatic>,
    /// Other players who still hold fanatics in front of them.
    pub rivals_with_fanatics: usize,
    /// Every peddler and caravan of the AI, with its load.
    pub pawns: Vec<MovablePawn>,
    /// The other players' peddlers and caravans.
    pub rivals: Vec<RivalPawn>,
    /// Sites that have already bought an item this turn.
    pub sold_sites: Vec<SiteId>,
}

impl AiOutlook {
    /// Turns still to play after this one.
    pub fn turns_after(&self) -> i64 {
        (self.max_turns - self.turn).max(0)
    }

    /// What the items are worth to the AI on the current board.
    pub fn item_values(&self) -> ItemValues {
        ItemValues::build(&self.board, &self.pawns, self.sale_turns)
    }

    /// The AI's pawns standing on `location`.
    pub fn own_pawns_on(&self, location: Location) -> usize {
        self.pawns
            .iter()
            .filter(|pawn| pawn.location == location)
            .count()
    }

    /// The rivals' pawns standing on `location`.
    pub fn rival_pawns_on(&self, location: Location) -> usize {
        self.rivals
            .iter()
            .filter(|pawn| pawn.location == location)
            .count()
    }
}

#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;

    /// An AI at turn 1 of 8, with one laboratory, 5,000 écus and nothing else.
    pub(crate) fn outlook() -> AiOutlook {
        AiOutlook {
            turn: 1,
            max_turns: 8,
            sale_turns: 8,
            board: BoardState::default(),
            money: 5_000,
            laboratories: 1,
            total_laboratories: 6,
            stock: ItemCounts::default(),
            supply: ItemCounts::initial_supply(),
            hand: Vec::new(),
            own_in_hall: None,
            others_in_hall: 0,
            unseen: Vec::new(),
            rivals_with_fanatics: 0,
            pawns: Vec::new(),
            rivals: Vec::new(),
            sold_sites: Vec::new(),
        }
    }

    /// A pawn of the AI with id `id`.
    pub(crate) fn pawn(
        id: i64,
        kind: PawnKind,
        location: Location,
        item: Option<crate::domain::MagicItem>,
    ) -> MovablePawn {
        MovablePawn {
            id,
            kind,
            location,
            item,
        }
    }
}
