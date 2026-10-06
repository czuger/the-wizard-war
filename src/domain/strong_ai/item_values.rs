//! What the items of a stock are worth to the AI by the end of the game.
//!
//! An item is worth its best map price, minus the trip, if one of the AI's pawns can take
//! it out before the end; otherwise it is worth the city price, discounted because the city
//! only buys when the collective expense opens it.

use crate::domain::ai_trading::MovablePawn;
use crate::domain::board::site_by_id;
use crate::domain::market::{buys_without_limit, city_price};
use crate::domain::strong_ai::buyer::buyers;
use crate::domain::{BoardState, ItemCounts, Location, MagicItem, PawnKind};

/// Chance that the city buys an item the pawns cannot take out.
const CITY_SALE_ODDS: f64 = 0.5;

/// Items a monastery is counted as buying in one turn (it has no limit).
const MONASTERY_SALES_PER_TURN: i64 = 3;

/// Turns a pawn of each kind needs for a typical trip out and back.
fn round_trip_turns(kind: PawnKind) -> i64 {
    match kind {
        PawnKind::Peddler => 4,
        PawnKind::Caravan => 2,
    }
}

/// The worth of each kind of item for one AI, on one board.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemValues {
    /// Best map value (price minus the trip) of each kind, in [`MagicItem::ALL`] order.
    map: [Option<i64>; 3],
    /// Items of each kind the map can buy before the end.
    outlets: [i64; 3],
    /// Items the AI's pawns can take out of the city before the end.
    trips: i64,
}

impl ItemValues {
    /// Values the items for an AI owning `pawns`.
    ///
    /// # Arguments
    /// * `sale_turns` - phases V left in the game, this turn's included if it is still ahead.
    pub fn build(board: &BoardState, pawns: &[MovablePawn], sale_turns: i64) -> ItemValues {
        let kind = if pawns.iter().any(|pawn| pawn.kind == PawnKind::Caravan) {
            PawnKind::Caravan
        } else {
            PawnKind::Peddler
        };
        let mut map = [None; 3];
        let mut outlets = [0; 3];
        for (index, item) in MagicItem::ALL.into_iter().enumerate() {
            let found = buyers(board, item, Location::City, kind, sale_turns);
            map[index] = found.iter().map(|buyer| buyer.score()).max();
            let per_turn: i64 = found
                .iter()
                .filter_map(|buyer| site_by_id(buyer.site))
                .map(|site| {
                    if buys_without_limit(site) {
                        MONASTERY_SALES_PER_TURN
                    } else {
                        1
                    }
                })
                .sum();
            outlets[index] = per_turn * sale_turns.max(0);
        }
        ItemValues {
            map,
            outlets,
            trips: trips_left(pawns, sale_turns),
        }
    }

    /// Best map value of `item`, if a pawn can still sell it on the map.
    pub fn map_value(&self, item: MagicItem) -> Option<i64> {
        self.map[index_of(item)]
    }

    /// Items the AI's pawns can still take out of the city.
    pub fn trips(&self) -> i64 {
        self.trips
    }

    /// Worth of `stock` in écus: the best items go out with the pawns, the rest to the city.
    pub fn stock_value(&self, stock: &ItemCounts) -> i64 {
        let fallback: i64 = MagicItem::ALL
            .into_iter()
            .map(|item| stock.get(item).max(0) * city_value(item))
            .sum();
        fallback + self.carried_gains(stock).iter().sum::<i64>()
    }

    /// Items of `stock` that no pawn will take out, left for the city.
    pub fn surplus(&self, stock: &ItemCounts) -> i64 {
        (stock.total() - self.carried_gains(stock).len() as i64).max(0)
    }

    /// What going out with a pawn adds to each item that gets a trip, best first.
    fn carried_gains(&self, stock: &ItemCounts) -> Vec<i64> {
        let mut gains: Vec<i64> = MagicItem::ALL
            .into_iter()
            .flat_map(|item| {
                let gain = self
                    .map_value(item)
                    .map_or(0, |value| value - city_value(item));
                let count = stock.get(item).clamp(0, self.outlets[index_of(item)]);
                std::iter::repeat_n(gain, count as usize)
            })
            .filter(|&gain| gain > 0)
            .collect();
        gains.sort_unstable_by(|a, b| b.cmp(a));
        gains.truncate(self.trips.max(0) as usize);
        gains
    }
}

/// The city price, discounted by the chance the city buys.
fn city_value(item: MagicItem) -> i64 {
    (city_price(item) as f64 * CITY_SALE_ODDS) as i64
}

fn index_of(item: MagicItem) -> usize {
    match item {
        MagicItem::Toratsa => 0,
        MagicItem::Xephis => 1,
        MagicItem::Yaboul => 2,
    }
}

/// Trips the pawns can still make with an item of the stock. Without pawns, the AI is
/// counted as buying a peddler soon.
fn trips_left(pawns: &[MovablePawn], sale_turns: i64) -> i64 {
    if sale_turns <= 0 {
        return 0;
    }
    if pawns.is_empty() {
        return (sale_turns - 1).max(0) / round_trip_turns(PawnKind::Peddler)
            + i64::from(sale_turns > 1);
    }
    pawns
        .iter()
        .map(|pawn| {
            let trips = (sale_turns / round_trip_turns(pawn.kind)).max(1);
            let busy = pawn.location != Location::City && pawn.item.is_some();
            if busy { trips - 1 } else { trips }
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::domain::{Cult, CultMarker, SiteId};

    fn board(markers: &[(u8, Cult)]) -> BoardState {
        let markers = markers
            .iter()
            .map(|&(site, cult)| (SiteId(site), CultMarker::face_up(cult)))
            .collect::<BTreeMap<_, _>>();
        BoardState::new(markers, Vec::new())
    }

    fn peddler(id: i64) -> MovablePawn {
        MovablePawn {
            id,
            kind: PawnKind::Peddler,
            location: Location::City,
            item: None,
        }
    }

    #[test]
    fn items_with_a_trip_are_worth_their_map_price() {
        let values = ItemValues::build(&board(&[(27, Cult::Yaboul)]), &[peddler(1)], 4);
        assert_eq!(values.map_value(MagicItem::Yaboul), Some(5_000 - 1_500));
        assert_eq!(values.map_value(MagicItem::Xephis), None);
        assert_eq!(values.trips(), 1);
        let one = values.stock_value(&ItemCounts::new(0, 0, 1));
        assert_eq!(one, 3_500);
        let two = values.stock_value(&ItemCounts::new(0, 0, 2));
        assert_eq!(two - one, 1_750, "the second item waits for the city");
        assert_eq!(values.surplus(&ItemCounts::new(0, 0, 2)), 1);
    }

    #[test]
    fn nothing_sells_on_the_map_after_the_last_phase_five() {
        let values = ItemValues::build(&board(&[(27, Cult::Yaboul)]), &[peddler(1)], 0);
        assert_eq!(values.map_value(MagicItem::Yaboul), None);
        assert_eq!(values.trips(), 0);
        assert_eq!(values.surplus(&ItemCounts::new(1, 0, 1)), 2);
    }

    #[test]
    fn loaded_pawns_on_the_map_are_busy() {
        let busy = MovablePawn {
            location: Location::Site(SiteId(27)),
            item: Some(MagicItem::Yaboul),
            ..peddler(1)
        };
        assert_eq!(trips_left(&[busy], 3), 0);
        assert_eq!(trips_left(&[peddler(1), peddler(2)], 8), 4);
        assert_eq!(trips_left(&[], 3), 1);
        assert_eq!(trips_left(&[], 1), 0);
    }
}
