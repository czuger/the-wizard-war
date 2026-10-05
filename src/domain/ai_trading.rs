//! Decisions of the computer-controlled alchemists in phases II.2 (fights), V, VI and VII.
//!
//! A pawn in the city loads the stock item that sells best near the city and heads for
//! the best buyer; a loaded pawn on a buying site stays and sells; an empty pawn, or one
//! whose item nobody buys, goes home. Pawns never stop on rebels. In the city, an AI sells
//! whenever it is asked. At the bank, it buys one pawn per laboratory, preferring caravans,
//! then laboratories, keeping some écus aside; it never sells pawns back.

use rand::Rng;
use rand::seq::IndexedRandom;

use crate::domain::board::SITES;
use crate::domain::market::{city_price, sale_price};
use crate::domain::rules::LABORATORY_PRICE;
use crate::domain::travel::{distance, step_toward};
use crate::domain::{
    BoardState, Holdings, ItemCounts, Location, MagicItem, PawnKind, PawnOrder, Purchase,
};

/// Écus an AI keeps after buying.
const RESERVE: i64 = 1_000;

/// Écus a buyer must pay above another, per extra step, to be worth the trip.
const STEP_VALUE: i64 = 1_500;

/// A peddler or caravan of the AI that may move this turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MovablePawn {
    /// Id of the pawn.
    pub id: i64,
    /// Peddler or caravan.
    pub kind: PawnKind,
    /// Where it stands.
    pub location: Location,
    /// The item it carries.
    pub item: Option<MagicItem>,
}

/// The best place to sell `item` seen from `from`, with its score.
fn best_buyer(board: &BoardState, from: Location, item: MagicItem) -> Option<(Location, i64)> {
    SITES
        .iter()
        .filter_map(|site| {
            let price = sale_price(site, board.marker(site.id), item)?;
            let target = Location::Site(site.id);
            let steps = distance(from, target)? as i64;
            Some((target, price - steps * STEP_VALUE))
        })
        .max_by_key(|(_, score)| *score)
}

/// Chooses the moves of the AI's movable pawns.
///
/// # Arguments
/// * `stock` - the AI's stock, from which pawns leaving the city load their item.
///
/// # Returns
/// One order per pawn that moves; the others stay where they are.
pub fn choose_pawn_moves(
    board: &BoardState,
    pawns: &[MovablePawn],
    stock: ItemCounts,
) -> Vec<PawnOrder> {
    let mut stock = stock;
    let mut orders = Vec::new();
    for pawn in pawns {
        let Some(order) = choose_pawn_move(board, pawn, stock) else {
            continue;
        };
        if let Some(item) = order.item {
            stock = stock.with(item, stock.get(item) - 1);
        }
        orders.push(order);
    }
    orders
}

/// Chooses the move of one pawn, or `None` to leave it where it is.
fn choose_pawn_move(
    board: &BoardState,
    pawn: &MovablePawn,
    stock: ItemCounts,
) -> Option<PawnOrder> {
    let (target, item) = match (pawn.location, pawn.item) {
        (Location::City, _) => {
            let (item, target) = choose_cargo(board, stock)?;
            (target, Some(item))
        }
        (Location::Site(_), Some(item)) => {
            let target =
                best_buyer(board, pawn.location, item).map_or(Location::City, |(target, _)| target);
            (target, None)
        }
        (Location::Site(_), None) => (Location::City, None),
    };
    let to = step_toward(pawn.kind, pawn.location, target);
    let blocked = to.site().is_some_and(|site| board.rebels_on(site) > 0);
    (to != pawn.location && !blocked).then_some(PawnOrder {
        pawn_id: pawn.id,
        to,
        item,
    })
}

/// The stock item that sells best seen from the city, and where.
fn choose_cargo(board: &BoardState, stock: ItemCounts) -> Option<(MagicItem, Location)> {
    MagicItem::ALL
        .into_iter()
        .filter(|&item| stock.get(item) > 0)
        .filter_map(|item| {
            best_buyer(board, Location::City, item).map(|(target, score)| (item, target, score))
        })
        .max_by_key(|(_, _, score)| *score)
        .map(|(item, target, _)| (item, target))
}

/// Chooses the item to sell in the city.
///
/// # Returns
/// The kind the AI holds most of, the best paid first on a tie; `None` with an empty stock.
pub fn choose_city_sale(stock: ItemCounts) -> Option<MagicItem> {
    MagicItem::ALL
        .into_iter()
        .filter(|&item| stock.get(item) > 0)
        .max_by_key(|&item| (stock.get(item), city_price(item)))
}

/// Chooses what the AI buys in phase VII: at most one pawn or laboratory.
///
/// # Arguments
/// * `laboratories` - laboratories the AI owns.
pub fn choose_purchase(holdings: &Holdings, laboratories: i64) -> Purchase {
    let affordable = |price: i64| holdings.money >= price + RESERVE;
    let pawns = holdings.peddlers + holdings.caravans;
    let purchase = if pawns < laboratories {
        if holdings.caravans < PawnKind::Caravan.max_owned() as i64
            && affordable(PawnKind::Caravan.price())
        {
            Purchase {
                caravans: 1,
                ..Purchase::default()
            }
        } else if holdings.peddlers < PawnKind::Peddler.max_owned() as i64
            && affordable(PawnKind::Peddler.price())
        {
            Purchase {
                peddlers: 1,
                ..Purchase::default()
            }
        } else {
            Purchase::default()
        }
    } else if holdings.laboratories_left > 0 && affordable(LABORATORY_PRICE) {
        Purchase {
            laboratories: 1,
            ..Purchase::default()
        }
    } else {
        Purchase::default()
    };
    if purchase.check(holdings).is_ok() {
        purchase
    } else {
        Purchase::default()
    }
}

/// Chooses which pawn a rebels pawn attacks: a rival's if there is one.
///
/// # Arguments
/// * `targets` - the pawns on the site, with whether they belong to the AI.
///
/// # Returns
/// The id of the attacked pawn, or `None` when there is none.
pub fn choose_fight_target<R: Rng>(targets: &[(i64, bool)], rng: &mut R) -> Option<i64> {
    let rivals: Vec<i64> = targets
        .iter()
        .filter(|(_, own)| !own)
        .map(|(id, _)| *id)
        .collect();
    let all: Vec<i64> = targets.iter().map(|(id, _)| *id).collect();
    let pool = if rivals.is_empty() { all } else { rivals };
    pool.choose(rng).copied()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::domain::{Cult, CultMarker, SiteId};

    fn board_with(markers: &[(u8, Cult)], rebels: &[u8]) -> BoardState {
        let markers = markers
            .iter()
            .map(|&(site, cult)| (SiteId(site), CultMarker::face_up(cult)))
            .collect::<BTreeMap<_, _>>();
        BoardState::new(markers, rebels.iter().map(|&site| SiteId(site)).collect())
    }

    fn pawn(location: Location, item: Option<MagicItem>) -> MovablePawn {
        MovablePawn {
            id: 1,
            kind: PawnKind::Peddler,
            location,
            item,
        }
    }

    #[test]
    fn city_pawns_load_an_item_that_sells() {
        let board = board_with(&[(27, Cult::Yaboul)], &[]);
        let stock = ItemCounts::new(0, 2, 1);
        let orders = choose_pawn_moves(&board, &[pawn(Location::City, None)], stock);
        assert_eq!(
            orders,
            vec![PawnOrder {
                pawn_id: 1,
                to: Location::Site(SiteId(27)),
                item: Some(MagicItem::Yaboul),
            }]
        );
        let nothing_sells =
            choose_pawn_moves(&BoardState::default(), &[pawn(Location::City, None)], stock);
        assert!(nothing_sells.is_empty());
    }

    #[test]
    fn loaded_pawns_stay_on_buyers_and_empty_ones_go_home() {
        let board = board_with(&[(27, Cult::Yaboul)], &[]);
        let at_27 = Location::Site(SiteId(27));
        let staying = pawn(at_27, Some(MagicItem::Yaboul));
        assert!(choose_pawn_moves(&board, &[staying], ItemCounts::default()).is_empty());
        let empty = pawn(at_27, None);
        assert_eq!(
            choose_pawn_moves(&board, &[empty], ItemCounts::default())[0].to,
            Location::City
        );
    }

    #[test]
    fn pawns_never_stop_on_rebels() {
        let board = board_with(&[(27, Cult::Yaboul)], &[27]);
        let orders = choose_pawn_moves(
            &board,
            &[pawn(Location::City, None)],
            ItemCounts::new(0, 0, 1),
        );
        assert!(orders.is_empty());
    }

    #[test]
    fn buys_a_pawn_per_laboratory_then_laboratories() {
        let holdings = Holdings {
            money: 8_000,
            peddlers: 0,
            caravans: 0,
            peddlers_in_city: 0,
            caravans_in_city: 0,
            laboratories_left: 15,
            bank_open: true,
        };
        assert_eq!(choose_purchase(&holdings, 1).caravans, 1);
        let poorer = Holdings {
            money: 5_000,
            ..holdings
        };
        assert_eq!(choose_purchase(&poorer, 1).peddlers, 1);
        let equipped = Holdings {
            money: 20_000,
            caravans: 1,
            ..holdings
        };
        assert_eq!(choose_purchase(&equipped, 1).laboratories, 1);
        assert!(
            choose_purchase(
                &Holdings {
                    money: 3_500,
                    ..holdings
                },
                1
            )
            .is_empty()
        );
    }

    #[test]
    fn sells_and_attacks() {
        assert_eq!(
            choose_city_sale(ItemCounts::new(1, 1, 0)),
            Some(MagicItem::Xephis)
        );
        assert_eq!(choose_city_sale(ItemCounts::default()), None);
        let mut rng = rand::rng();
        assert_eq!(
            choose_fight_target(&[(1, true), (2, false)], &mut rng),
            Some(2)
        );
        assert_eq!(choose_fight_target(&[(1, true)], &mut rng), Some(1));
        assert_eq!(choose_fight_target(&[], &mut rng), None);
    }
}
