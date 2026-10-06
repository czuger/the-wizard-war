//! Phase III: which items the AI makes, weighing what the fanatics may do in phase IV.

use crate::domain::strong_ai::fanatic_odds::FanaticOdds;
use crate::domain::strong_ai::item_values::ItemValues;
use crate::domain::strong_ai::outlook::AiOutlook;
use crate::domain::{Cult, FanaticVerdict, ItemCounts, MagicItem};

/// Chooses the `count` items to take from the supply.
///
/// Every split of `count` items that fits the supply is scored by the expected worth of the
/// stock once phase IV is over, given the odds of each Dominant Fanatic with the pawns now in
/// the hall. The verdict rules come from [`FanaticVerdict`].
///
/// # Returns
/// The best split; it always fits the supply and holds `count` items when the supply has
/// that many.
pub fn choose_production(outlook: &AiOutlook, count: i64) -> ItemCounts {
    let values = outlook.item_values();
    let known: Vec<_> = outlook.own_in_hall.into_iter().collect();
    let odds = FanaticOdds::compute(&known, &outlook.unseen, outlook.others_in_hall);
    let count = count.clamp(0, outlook.supply.total());
    let mut best = (f64::MIN, ItemCounts::default());
    for split in splits(count, &outlook.supply) {
        let worth = expected_worth(outlook, &values, &odds, split);
        if worth > best.0 {
            best = (worth, split);
        }
    }
    best.1
}

/// Every way to take `count` items within `supply`, talismans first.
fn splits(count: i64, supply: &ItemCounts) -> Vec<ItemCounts> {
    let mut found = Vec::new();
    for toratsa in (0..=count.min(supply.toratsa)).rev() {
        for xephis in (0..=(count - toratsa).min(supply.xephis)).rev() {
            let yaboul = count - toratsa - xephis;
            if yaboul <= supply.yaboul {
                found.push(ItemCounts::new(toratsa, xephis, yaboul));
            }
        }
    }
    found
}

/// Expected worth of the stock after phase IV if the AI makes `production`.
fn expected_worth(
    outlook: &AiOutlook,
    values: &ItemValues,
    odds: &FanaticOdds,
    production: ItemCounts,
) -> f64 {
    let left = outlook.supply.minus(production);
    [Some(Cult::Xephis), Some(Cult::Yaboul), None]
        .into_iter()
        .map(|dominant| {
            let verdict = dominant.map_or(FanaticVerdict::Unaffected, |cult| {
                FanaticVerdict::judge(cult, &production)
            });
            let bonus_available = verdict
                .bonus_item()
                .is_some_and(|item: MagicItem| left.get(item) > 0);
            let stock = verdict.new_stock(production, outlook.stock, bonus_available);
            odds.of(dominant) * values.stock_value(&stock) as f64
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::domain::strong_ai::outlook::fixtures::{outlook, pawn};
    use crate::domain::{BoardState, CultMarker, Fanatic, Location, PawnKind, SiteId};

    fn selling_board() -> BoardState {
        let markers = [(1, Cult::Yaboul), (2, Cult::Xephis), (27, Cult::Yaboul)]
            .into_iter()
            .map(|(site, cult)| (SiteId(site), CultMarker::face_up(cult)))
            .collect::<BTreeMap<_, _>>();
        BoardState::new(markers, Vec::new())
    }

    #[test]
    fn makes_the_dominant_cult_when_it_is_certain() {
        let mut outlook = outlook();
        outlook.board = selling_board();
        outlook.pawns = vec![pawn(1, PawnKind::Caravan, Location::City, None)];
        outlook.own_in_hall = Some(Fanatic::new(Cult::Yaboul, 3));
        assert_eq!(choose_production(&outlook, 2), ItemCounts::new(0, 0, 2));
    }

    #[test]
    fn avoids_the_cult_likely_to_be_raided() {
        let mut outlook = outlook();
        outlook.board = selling_board();
        outlook.pawns = vec![pawn(1, PawnKind::Caravan, Location::City, None)];
        outlook.stock = ItemCounts::new(0, 0, 3);
        outlook.unseen = vec![Fanatic::new(Cult::Xephis, 3); 3];
        outlook.others_in_hall = 1;
        let made = choose_production(&outlook, 2);
        assert_eq!(made.yaboul, 0, "Xéphis will raid Yaboul makers: {made:?}");
    }

    #[test]
    fn production_fits_the_supply() {
        let mut outlook = outlook();
        outlook.supply = ItemCounts::new(1, 0, 1);
        assert_eq!(choose_production(&outlook, 5).total(), 2);
        assert!(choose_production(&outlook, 5).fits_within(&outlook.supply));
        assert_eq!(splits(2, &ItemCounts::new(5, 5, 5)).len(), 6);
    }
}
