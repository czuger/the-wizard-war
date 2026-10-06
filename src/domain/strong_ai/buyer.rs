//! Where a pawn can sell an item, and how long it takes to get there.

use crate::domain::board::SITES;
use crate::domain::market::sale_price;
use crate::domain::travel::distance;
use crate::domain::{BoardState, Location, MagicItem, PawnKind, SiteId};

/// What a turn of travel costs, in écus: the pawn is busy, and the target may change cult
/// before it arrives.
pub const TURN_COST: i64 = 1_500;

/// A site that buys an item, seen from a pawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Buyer {
    /// The buying site.
    pub site: SiteId,
    /// What it pays, from Table 1.
    pub price: i64,
    /// Turns the pawn needs to get there; 0 when it already stands on it.
    pub turns: i64,
}

impl Buyer {
    /// The price minus the cost of the trip.
    pub fn score(&self) -> i64 {
        self.price - TURN_COST * self.turns
    }
}

/// Turns a pawn of `kind` needs to go from `from` to `to`.
///
/// # Returns
/// `None` when `to` cannot be reached.
pub fn travel_turns(kind: PawnKind, from: Location, to: Location) -> Option<i64> {
    let steps = distance(from, to)? as i64;
    let speed = kind.speed() as i64;
    Some((steps + speed - 1) / speed)
}

/// Every site that buys `item` and that a pawn of `kind` on `from` reaches in time to sell.
///
/// # Arguments
/// * `sale_turns` - phases V left in the game, this turn's included if it is still ahead.
pub fn buyers(
    board: &BoardState,
    item: MagicItem,
    from: Location,
    kind: PawnKind,
    sale_turns: i64,
) -> Vec<Buyer> {
    SITES
        .iter()
        .filter_map(|site| {
            let price = sale_price(site, board.marker(site.id), item)?;
            let turns = travel_turns(kind, from, Location::Site(site.id))?;
            (turns.max(1) <= sale_turns).then_some(Buyer {
                site: site.id,
                price,
                turns,
            })
        })
        .collect()
}

/// The buyer with the best [`Buyer::score`], leaving out the `excluded` sites.
pub fn best_buyer(
    board: &BoardState,
    item: MagicItem,
    from: Location,
    kind: PawnKind,
    sale_turns: i64,
    excluded: &[SiteId],
) -> Option<Buyer> {
    buyers(board, item, from, kind, sale_turns)
        .into_iter()
        .filter(|buyer| !excluded.contains(&buyer.site))
        .max_by_key(|buyer| (buyer.score(), buyer.price))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::domain::{Cult, CultMarker};

    fn board(markers: &[(u8, Cult)]) -> BoardState {
        let markers = markers
            .iter()
            .map(|&(site, cult)| (SiteId(site), CultMarker::face_up(cult)))
            .collect::<BTreeMap<_, _>>();
        BoardState::new(markers, Vec::new())
    }

    #[test]
    fn caravans_travel_two_steps_a_turn() {
        let to_16 = Location::Site(SiteId(16));
        assert_eq!(
            travel_turns(PawnKind::Peddler, Location::City, to_16),
            Some(2)
        );
        assert_eq!(
            travel_turns(PawnKind::Caravan, Location::City, to_16),
            Some(1)
        );
        assert_eq!(travel_turns(PawnKind::Caravan, to_16, to_16), Some(0));
    }

    #[test]
    fn far_buyers_must_be_reached_before_the_end() {
        let board = board(&[(27, Cult::Yaboul), (1, Cult::Yaboul)]);
        let far = |sale_turns| {
            best_buyer(
                &board,
                MagicItem::Yaboul,
                Location::City,
                PawnKind::Caravan,
                sale_turns,
                &[],
            )
        };
        assert_eq!(far(9).map(|buyer| buyer.site), Some(SiteId(1)));
        assert_eq!(far(1).map(|buyer| buyer.site), Some(SiteId(27)));
        assert_eq!(far(0), None);
        let excluded = best_buyer(
            &board,
            MagicItem::Yaboul,
            Location::City,
            PawnKind::Caravan,
            1,
            &[SiteId(27)],
        );
        assert_eq!(excluded, None);
    }

    #[test]
    fn castles_buy_talismans() {
        let found = buyers(
            &BoardState::default(),
            MagicItem::Toratsa,
            Location::City,
            PawnKind::Peddler,
            9,
        );
        let sites: Vec<SiteId> = found.iter().map(|buyer| buyer.site).collect();
        assert_eq!(sites, vec![SiteId(10), SiteId(21)]);
    }
}
