//! Decisions of the computer-controlled alchemists.
//!
//! The strategy is deliberately basic: an AI backs the cult its fanatics are strongest in,
//! sends a pawn of that cult to the hall one turn in two, and only makes items of that cult
//! when it has sent a pawn. Otherwise it makes Toratsa talismans, which the fanatics ignore.
//! It only uses what the player could see: its own hand, its money and the supply.

use rand::{Rng, RngExt};

use crate::domain::rules::EXPENSE_STEP;
use crate::domain::{Cult, Fanatic, ItemCounts, MagicItem};

/// Chance that an AI sends a fanatic to the hall in a given turn.
const SEND_PROBABILITY: f64 = 0.5;

/// Largest expense an AI pays in one turn, in écus.
const MAX_EXPENSE: i64 = 300;

/// Returns the cult with the most total strength in `hand`, Xéphis on a tie.
///
/// # Returns
/// `None` when the hand is empty.
pub fn preferred_cult(hand: &[Fanatic]) -> Option<Cult> {
    let strength_of = |cult: Cult| -> i64 {
        hand.iter()
            .filter(|fanatic| fanatic.cult == cult)
            .map(|fanatic| fanatic.strength)
            .sum()
    };
    if hand.is_empty() {
        return None;
    }
    if strength_of(Cult::Yaboul) > strength_of(Cult::Xephis) {
        Some(Cult::Yaboul)
    } else {
        Some(Cult::Xephis)
    }
}

/// Chooses the fanatic to send to the great hall this turn.
///
/// # Returns
/// The index in `hand` of the strongest pawn of the preferred cult, or `None` to send nothing.
pub fn choose_announcement<R: Rng>(hand: &[Fanatic], rng: &mut R) -> Option<usize> {
    let cult = preferred_cult(hand)?;
    if !rng.random_bool(SEND_PROBABILITY) {
        return None;
    }
    hand.iter()
        .enumerate()
        .filter(|(_, fanatic)| fanatic.cult == cult)
        .max_by_key(|(_, fanatic)| fanatic.strength)
        .map(|(index, _)| index)
}

/// Chooses a random expense between 0 and 300 écus, never more than `money`.
pub fn choose_expense<R: Rng>(money: i64, rng: &mut R) -> i64 {
    let most = MAX_EXPENSE.min(money.max(0)) / EXPENSE_STEP;
    rng.random_range(0..=most) * EXPENSE_STEP
}

/// Chooses `count` items to take from `supply`.
///
/// # Arguments
/// * `backed_cult` - the cult of the fanatic the AI sent to the hall this turn, if any.
///
/// # Returns
/// Items of the backed cult first, then talismans, then whatever is left. Without a backed
/// cult, talismans first, then the cult item the supply holds most of. The result always
/// fits within `supply` and holds `count` items as long as the supply has that many.
pub fn choose_production(count: i64, supply: &ItemCounts, backed_cult: Option<Cult>) -> ItemCounts {
    let order = match backed_cult {
        Some(cult) => [
            MagicItem::of_cult(cult),
            MagicItem::Toratsa,
            MagicItem::of_cult(cult.opposite()),
        ],
        None if supply.yaboul > supply.xephis => {
            [MagicItem::Toratsa, MagicItem::Yaboul, MagicItem::Xephis]
        }
        None => [MagicItem::Toratsa, MagicItem::Xephis, MagicItem::Yaboul],
    };
    let mut left = count;
    let mut chosen = ItemCounts::default();
    for item in order {
        let taken = left.min(supply.get(item));
        chosen = chosen.with(item, taken);
        left -= taken;
    }
    chosen
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_the_strongest_cult() {
        let hand = [
            Fanatic::new(Cult::Xephis, 1),
            Fanatic::new(Cult::Xephis, 1),
            Fanatic::new(Cult::Yaboul, 3),
        ];
        assert_eq!(preferred_cult(&hand), Some(Cult::Yaboul));
        assert_eq!(preferred_cult(&[]), None);
    }

    #[test]
    fn announcement_is_the_strongest_pawn_of_the_preferred_cult() {
        let hand = [
            Fanatic::new(Cult::Xephis, 1),
            Fanatic::new(Cult::Yaboul, 1),
            Fanatic::new(Cult::Xephis, 3),
        ];
        let mut rng = rand::rng();
        for _ in 0..50 {
            let choice = choose_announcement(&hand, &mut rng);
            assert!(matches!(choice, None | Some(2)));
        }
        assert_eq!(choose_announcement(&[], &mut rng), None);
    }

    #[test]
    fn expense_is_affordable_and_in_steps() {
        let mut rng = rand::rng();
        for money in [0, 50, 100, 250, 5_000] {
            let expense = choose_expense(money, &mut rng);
            assert!((0..=money.min(MAX_EXPENSE)).contains(&expense));
            assert_eq!(expense % EXPENSE_STEP, 0);
        }
    }

    #[test]
    fn production_follows_the_backed_cult() {
        let supply = ItemCounts::new(5, 5, 5);
        assert_eq!(
            choose_production(2, &supply, Some(Cult::Yaboul)),
            ItemCounts::new(0, 0, 2)
        );
        assert_eq!(
            choose_production(2, &supply, None),
            ItemCounts::new(2, 0, 0)
        );
    }

    #[test]
    fn production_falls_back_when_the_supply_runs_out() {
        let supply = ItemCounts::new(1, 0, 3);
        assert_eq!(
            choose_production(3, &supply, Some(Cult::Xephis)),
            ItemCounts::new(1, 0, 2)
        );
        assert_eq!(
            choose_production(3, &supply, None),
            ItemCounts::new(1, 0, 2)
        );
        assert_eq!(
            choose_production(9, &supply, None),
            ItemCounts::new(1, 0, 3)
        );
    }
}
