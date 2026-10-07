//! Phase III: which items the AI makes, weighing what the fanatics may do in phase IV.

use rand::Rng;

use crate::domain::strong_ai::fanatic_odds::FanaticOdds;
use crate::domain::strong_ai::item_values::ItemValues;
use crate::domain::strong_ai::near_best::pick_near_best;
use crate::domain::strong_ai::outlook::AiOutlook;
use crate::domain::{Cult, FanaticVerdict, ItemCounts, MagicItem};

/// Chooses the `count` items to take from the supply.
///
/// Every split of `count` items that fits the supply is scored by the expected worth of the
/// stock once phase IV is over, given the odds of each Dominant Fanatic with the pawns now in
/// the hall, minus the AI's aversion to the spread of that worth: a raid can destroy the
/// cult items, never the talismans. The verdict rules come from [`FanaticVerdict`].
///
/// # Returns
/// One of the best splits; it always fits the supply and holds `count` items when the
/// supply has that many.
pub fn choose_production<R: Rng>(outlook: &AiOutlook, count: i64, rng: &mut R) -> ItemCounts {
    let values = outlook.item_values();
    let known: Vec<_> = outlook.own_in_hall.into_iter().collect();
    let odds = FanaticOdds::compute(&known, &outlook.unseen, outlook.others_in_hall);
    let count = count.clamp(0, outlook.supply.total());
    let scored = splits(count, &outlook.supply)
        .into_iter()
        .map(|split| (split, score(outlook, &values, &odds, split)))
        .collect();
    let tolerance = outlook.temperament.tolerance as f64;
    pick_near_best(scored, tolerance, rng).unwrap_or_default()
}

/// The expected worth of the stock after `production`, minus the AI's risk aversion times
/// its standard deviation over the Dominant Fanatics.
fn score(
    outlook: &AiOutlook,
    values: &ItemValues,
    odds: &FanaticOdds,
    production: ItemCounts,
) -> f64 {
    let outcomes = outcomes(outlook, values, production);
    let mean: f64 = outcomes
        .iter()
        .map(|&(dominant, worth)| odds.of(dominant) * worth)
        .sum();
    let variance: f64 = outcomes
        .iter()
        .map(|&(dominant, worth)| odds.of(dominant) * (worth - mean).powi(2))
        .sum();
    mean - outlook.temperament.risk_aversion * variance.sqrt()
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

/// Worth of the stock after phase IV if the AI makes `production`, for each Dominant
/// Fanatic.
fn outcomes(
    outlook: &AiOutlook,
    values: &ItemValues,
    production: ItemCounts,
) -> [(Option<Cult>, f64); 3] {
    let left = outlook.supply.minus(production);
    [Some(Cult::Xephis), Some(Cult::Yaboul), None].map(|dominant| {
        let verdict = dominant.map_or(FanaticVerdict::Unaffected, |cult| {
            FanaticVerdict::judge(cult, &production)
        });
        let bonus_available = verdict
            .bonus_item()
            .is_some_and(|item: MagicItem| left.get(item) > 0);
        let stock = verdict.new_stock(production, outlook.stock, bonus_available);
        (dominant, values.stock_value(&stock) as f64)
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::domain::strong_ai::outlook::fixtures::{outlook, pawn};
    use crate::domain::{
        AiTemperament, Archetype, BoardState, CultMarker, Fanatic, Location, PawnKind, SiteId,
    };

    fn selling_board() -> BoardState {
        let markers = [(3, Cult::Yaboul), (2, Cult::Xephis), (27, Cult::Yaboul)]
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
        assert_eq!(
            choose_production(&outlook, 2, &mut rand::rng()),
            ItemCounts::new(0, 0, 2)
        );
    }

    #[test]
    fn avoids_the_cult_likely_to_be_raided() {
        let mut outlook = outlook();
        outlook.board = selling_board();
        outlook.pawns = vec![pawn(1, PawnKind::Caravan, Location::City, None)];
        outlook.stock = ItemCounts::new(0, 0, 3);
        outlook.unseen = vec![Fanatic::new(Cult::Xephis, 3); 3];
        outlook.others_in_hall = 1;
        let made = choose_production(&outlook, 2, &mut rand::rng());
        assert_eq!(made.yaboul, 0, "Xéphis will raid Yaboul makers: {made:?}");
    }

    #[test]
    fn production_fits_the_supply() {
        let mut outlook = outlook();
        outlook.supply = ItemCounts::new(1, 0, 1);
        assert_eq!(choose_production(&outlook, 5, &mut rand::rng()).total(), 2);
        assert!(choose_production(&outlook, 5, &mut rand::rng()).fits_within(&outlook.supply));
        assert_eq!(splits(2, &ItemCounts::new(5, 5, 5)).len(), 6);
    }

    #[test]
    fn a_merchant_makes_talismans_where_a_zealot_takes_the_risk() {
        let mut outlook = outlook();
        outlook.board = selling_board();
        outlook.pawns = vec![pawn(1, PawnKind::Caravan, Location::City, None)];
        outlook.unseen = vec![Fanatic::new(Cult::Xephis, 2), Fanatic::new(Cult::Yaboul, 2)];
        outlook.others_in_hall = 1;
        let mut made_by = |archetype| {
            outlook.temperament = AiTemperament {
                tolerance: 0,
                ..AiTemperament::new(archetype, 0)
            };
            choose_production(&outlook, 2, &mut rand::rng())
        };
        assert_eq!(made_by(Archetype::Merchant), ItemCounts::new(2, 0, 0));
        assert!(made_by(Archetype::YaboulZealot).yaboul > 0);
    }
}
