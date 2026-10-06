//! Phase V.1: where the AI's peddlers and caravans go, and what they carry.

use crate::domain::ai_trading::MovablePawn;
use crate::domain::board::{neighbours, site_by_id};
use crate::domain::market::buys_without_limit;
use crate::domain::strong_ai::buyer::{Buyer, buyers};
use crate::domain::strong_ai::outlook::AiOutlook;
use crate::domain::travel::{destinations, distance};
use crate::domain::{ItemCounts, Location, MagicItem, PawnKind, PawnOrder, SiteId};

/// Chooses the moves of the AI's movable pawns.
///
/// Loaded pawns on the map go first, then the pawns in the city (caravans first), then the
/// empty ones, which go home. Each loaded pawn heads for the buyer with the best price once
/// the trip is counted, among those it reaches before the end; a buyer that takes one item
/// a turn is kept for a single pawn of the AI. A pawn in the city leaves with the stock item
/// that has the best such buyer, and stays home when nothing can be sold in time. Every
/// pawn stops where it gets closest to its target without ending on rebels, away from
/// rebels when it can.
///
/// # Returns
/// One order per pawn that moves; the others stay where they are.
pub fn choose_pawn_moves(outlook: &AiOutlook, movable: &[MovablePawn]) -> Vec<PawnOrder> {
    let mut pawns = movable.to_vec();
    pawns.sort_by_key(|pawn| match (pawn.location, pawn.item) {
        (Location::Site(_), Some(_)) => 0,
        (Location::City, _) => 1 + i64::from(pawn.kind == PawnKind::Peddler),
        (Location::Site(_), None) => 3,
    });
    let mut planner = Planner {
        outlook,
        stock: outlook.stock,
        reserved: Vec::new(),
    };
    pawns.iter().filter_map(|pawn| planner.plan(pawn)).collect()
}

/// The orders being given, with what they have already taken.
struct Planner<'a> {
    outlook: &'a AiOutlook,
    /// Stock left to load.
    stock: ItemCounts,
    /// Buyers already chosen by another pawn of the AI.
    reserved: Vec<SiteId>,
}

impl Planner<'_> {
    /// The order of one pawn, or `None` to leave it where it is.
    fn plan(&mut self, pawn: &MovablePawn) -> Option<PawnOrder> {
        match (pawn.location, pawn.item) {
            (Location::Site(_), Some(item)) => {
                let Some(buyer) = self.pick_buyer(item, pawn) else {
                    return self.order(pawn, Location::City, None);
                };
                self.reserve(buyer.site);
                if buyer.turns == 0 {
                    return None;
                }
                self.order(pawn, Location::Site(buyer.site), None)
            }
            (Location::City, _) => {
                let (item, buyer) = MagicItem::ALL
                    .into_iter()
                    .filter(|&item| self.stock.get(item) > 0)
                    .filter_map(|item| Some((item, self.pick_buyer(item, pawn)?)))
                    .filter(|(_, buyer)| buyer.score() > 0)
                    .max_by_key(|(_, buyer)| (buyer.score(), buyer.price))?;
                let order = self.order(pawn, Location::Site(buyer.site), Some(item))?;
                self.reserve(buyer.site);
                self.stock = self.stock.with(item, self.stock.get(item) - 1);
                Some(order)
            }
            (Location::Site(_), None) => self.order(pawn, Location::City, None),
        }
    }

    /// The best buyer of `item` for `pawn`, leaving out those taken by other pawns of the AI
    /// and, when reached this turn, those that have already bought this turn.
    fn pick_buyer(&self, item: MagicItem, pawn: &MovablePawn) -> Option<Buyer> {
        let outlook = self.outlook;
        buyers(
            &outlook.board,
            item,
            pawn.location,
            pawn.kind,
            outlook.sale_turns,
        )
        .into_iter()
        .filter(|buyer| !self.reserved.contains(&buyer.site))
        .filter(|buyer| buyer.turns > 1 || !outlook.sold_sites.contains(&buyer.site))
        .max_by_key(|buyer| (buyer.score(), buyer.price))
    }

    /// Keeps `site` for one pawn, unless it buys without limit.
    fn reserve(&mut self, site: SiteId) {
        if site_by_id(site).is_some_and(|site| !buys_without_limit(site)) {
            self.reserved.push(site);
        }
    }

    /// The order sending `pawn` toward `target`, if it can get closer.
    fn order(
        &self,
        pawn: &MovablePawn,
        target: Location,
        item: Option<MagicItem>,
    ) -> Option<PawnOrder> {
        let to = next_stop(self.outlook, pawn, target)?;
        Some(PawnOrder {
            pawn_id: pawn.id,
            to,
            item,
        })
    }
}

/// The stop closest to `target` that `pawn` reaches this turn without ending on rebels,
/// the one with fewer rebels around on a tie.
///
/// # Returns
/// `None` when no such stop is closer than where the pawn stands.
fn next_stop(outlook: &AiOutlook, pawn: &MovablePawn, target: Location) -> Option<Location> {
    let board = &outlook.board;
    let now = distance(pawn.location, target)?;
    destinations(pawn.kind, pawn.location)
        .into_iter()
        .filter(|stop| stop.site().is_none_or(|site| board.rebels_on(site) == 0))
        .filter_map(|stop| {
            let left = distance(stop, target)?;
            let threat: usize = stop.site().map_or(0, |site| {
                neighbours(site)
                    .into_iter()
                    .map(|next| board.rebels_on(next))
                    .sum()
            });
            (left < now).then_some((stop, left, threat))
        })
        .min_by_key(|(_, left, threat)| (*left, *threat))
        .map(|(stop, _, _)| stop)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::domain::strong_ai::outlook::fixtures::{outlook, pawn};
    use crate::domain::{BoardState, Cult, CultMarker, ItemCounts};

    fn board(markers: &[(u8, Cult)], rebels: &[u8]) -> BoardState {
        let markers = markers
            .iter()
            .map(|&(site, cult)| (SiteId(site), CultMarker::face_up(cult)))
            .collect::<BTreeMap<_, _>>();
        BoardState::new(markers, rebels.iter().map(|&site| SiteId(site)).collect())
    }

    const AT_27: Location = Location::Site(SiteId(27));

    #[test]
    fn two_pawns_never_aim_at_the_same_village() {
        let mut outlook = outlook();
        outlook.board = board(&[(27, Cult::Yaboul), (28, Cult::Yaboul)], &[]);
        outlook.stock = ItemCounts::new(0, 0, 2);
        outlook.sale_turns = 1;
        let movable = [
            pawn(1, PawnKind::Peddler, Location::City, None),
            pawn(2, PawnKind::Peddler, Location::City, None),
        ];
        let orders = choose_pawn_moves(&outlook, &movable);
        assert_eq!(orders.len(), 2);
        assert_ne!(orders[0].to, orders[1].to);
        assert!(
            orders
                .iter()
                .all(|order| order.item == Some(MagicItem::Yaboul))
        );
    }

    #[test]
    fn stays_home_when_nothing_sells_in_time() {
        let mut outlook = outlook();
        outlook.board = board(&[(1, Cult::Yaboul)], &[]);
        outlook.stock = ItemCounts::new(0, 0, 1);
        outlook.sale_turns = 1;
        let movable = [pawn(1, PawnKind::Peddler, Location::City, None)];
        assert!(choose_pawn_moves(&outlook, &movable).is_empty());
    }

    #[test]
    fn loaded_pawns_sell_where_they_stand_and_avoid_rebels() {
        let mut outlook = outlook();
        outlook.board = board(&[(27, Cult::Yaboul)], &[]);
        let staying = pawn(1, PawnKind::Peddler, AT_27, Some(MagicItem::Yaboul));
        assert!(choose_pawn_moves(&outlook, &[staying]).is_empty());
        outlook.board = board(&[(27, Cult::Yaboul)], &[27]);
        let leaving = pawn(2, PawnKind::Peddler, Location::City, None);
        outlook.stock = ItemCounts::new(0, 0, 1);
        assert!(choose_pawn_moves(&outlook, &[leaving]).is_empty());
    }

    #[test]
    fn empty_pawns_go_home() {
        let outlook = outlook();
        let empty = pawn(1, PawnKind::Peddler, AT_27, None);
        let orders = choose_pawn_moves(&outlook, &[empty]);
        assert_eq!(orders[0].to, Location::City);
    }
}
