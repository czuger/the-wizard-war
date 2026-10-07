//! Decisions of the « Débutant » computer-controlled alchemists (see
//! [`crate::domain::strong_ai`] for the « Fort » ones).
//!
//! The strategy is deliberately basic: an AI backs the cult its fanatics are strongest in,
//! sends a pawn of that cult to the hall one turn in two, and only makes items of that cult
//! when it has sent a pawn. Otherwise it makes Toratsa talismans, which the fanatics ignore.
//! On the map it spreads that cult: it converts hidden rival villages, places its markers
//! (castles and monasteries first, since they never change), hides rival markers and reveals
//! its own. Rebels move at random.
//! It only uses what the player could see: its own hand, its money, the supply and the map.

use rand::seq::IndexedRandom;
use rand::{Rng, RngExt};

use crate::domain::board::site_by_id;
use crate::domain::rules::EXPENSE_STEP;
use crate::domain::{BoardState, Conversion, Cult, Fanatic, ItemCounts, MagicItem};

/// Chance that an AI sends a fanatic to the hall in a given turn.
const SEND_PROBABILITY: f64 = 0.5;

/// Largest expense an AI pays in one turn, in écus: enough, all together, to open the
/// city to sales in the first turns.
const MAX_EXPENSE: i64 = 1_000;

/// Returns the cult with the most total strength in `hand`, `tie` on a tie.
///
/// # Returns
/// `None` when the hand is empty.
pub fn preferred_cult(hand: &[Fanatic], tie: Cult) -> Option<Cult> {
    let strength_of = |cult: Cult| -> i64 {
        hand.iter()
            .filter(|fanatic| fanatic.cult == cult)
            .map(|fanatic| fanatic.strength)
            .sum()
    };
    if hand.is_empty() {
        return None;
    }
    match strength_of(Cult::Yaboul).cmp(&strength_of(Cult::Xephis)) {
        std::cmp::Ordering::Greater => Some(Cult::Yaboul),
        std::cmp::Ordering::Less => Some(Cult::Xephis),
        std::cmp::Ordering::Equal => Some(tie),
    }
}

/// Chooses the fanatic to send to the great hall this turn.
///
/// # Arguments
/// * `home_cult` - the cult the AI leans to on a tie.
///
/// # Returns
/// The index in `hand` of the strongest pawn of the preferred cult, or `None` to send nothing.
pub fn choose_announcement<R: Rng>(
    hand: &[Fanatic],
    home_cult: Cult,
    rng: &mut R,
) -> Option<usize> {
    let cult = preferred_cult(hand, home_cult)?;
    if !rng.random_bool(SEND_PROBABILITY) {
        return None;
    }
    hand.iter()
        .enumerate()
        .filter(|(_, fanatic)| fanatic.cult == cult)
        .max_by_key(|(_, fanatic)| fanatic.strength)
        .map(|(index, _)| index)
}

/// Chooses a random expense between 0 and 1,000 écus, never more than `money`.
pub fn choose_expense<R: Rng>(money: i64, rng: &mut R) -> i64 {
    let most = MAX_EXPENSE.min(money.max(0)) / EXPENSE_STEP;
    rng.random_range(0..=most) * EXPENSE_STEP
}

/// Chooses `count` items to take from `supply`.
///
/// # Arguments
/// * `backed_cult` - the cult of the fanatic the AI sent to the hall this turn, if any.
/// * `home_cult` - the cult the AI leans to on a tie.
///
/// # Returns
/// Items of the backed cult first, then talismans, then whatever is left. Without a backed
/// cult, talismans first, then the cult item the supply holds most of. The result always
/// fits within `supply` and holds `count` items as long as the supply has that many.
pub fn choose_production(
    count: i64,
    supply: &ItemCounts,
    backed_cult: Option<Cult>,
    home_cult: Cult,
) -> ItemCounts {
    let plentiful = match supply.yaboul.cmp(&supply.xephis) {
        std::cmp::Ordering::Greater => Cult::Yaboul,
        std::cmp::Ordering::Less => Cult::Xephis,
        std::cmp::Ordering::Equal => home_cult,
    };
    let cult = backed_cult.unwrap_or(plentiful);
    let order = if backed_cult.is_some() {
        [
            MagicItem::of_cult(cult),
            MagicItem::Toratsa,
            MagicItem::of_cult(cult.opposite()),
        ]
    } else {
        [
            MagicItem::Toratsa,
            MagicItem::of_cult(cult),
            MagicItem::of_cult(cult.opposite()),
        ]
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

/// Chooses the conversion of phase II.1 for an AI backing `cult`.
///
/// # Returns
/// A random conversion among the most useful legal ones, or `None` when none is legal.
pub fn choose_conversion<R: Rng>(
    board: &BoardState,
    turn: i64,
    cult: Option<Cult>,
    rng: &mut R,
) -> Option<Conversion> {
    let legal = board.legal_conversions(turn);
    let Some(cult) = cult else {
        return legal.choose(rng).copied();
    };
    let best = legal
        .iter()
        .map(|conversion| conversion_priority(board, cult, conversion))
        .min()?;
    let candidates: Vec<Conversion> = legal
        .into_iter()
        .filter(|conversion| conversion_priority(board, cult, conversion) == best)
        .collect();
    candidates.choose(rng).copied()
}

/// Ranks a legal conversion for an AI backing `cult`: the lower, the better.
pub fn conversion_priority(board: &BoardState, cult: Cult, conversion: &Conversion) -> u8 {
    let marker = board.marker(conversion.site());
    let marker_is = |wanted: Cult, face_up: bool| {
        marker.is_some_and(|marker| marker.cult == wanted && marker.face_up == face_up)
    };
    match *conversion {
        Conversion::Replace { .. } if marker_is(cult.opposite(), false) => 0,
        Conversion::Place { site, cult: placed } if placed == cult => {
            if site_by_id(site).is_some_and(|site| site.kind.is_permanent()) {
                1
            } else {
                2
            }
        }
        Conversion::Flip { .. } if marker_is(cult.opposite(), true) => 3,
        Conversion::Flip { .. } if marker_is(cult, false) => 4,
        _ => 5,
    }
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
        assert_eq!(preferred_cult(&hand, Cult::Xephis), Some(Cult::Yaboul));
        assert_eq!(preferred_cult(&[], Cult::Xephis), None);
        let even = [Fanatic::new(Cult::Xephis, 2), Fanatic::new(Cult::Yaboul, 2)];
        assert_eq!(preferred_cult(&even, Cult::Yaboul), Some(Cult::Yaboul));
        assert_eq!(preferred_cult(&even, Cult::Xephis), Some(Cult::Xephis));
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
            let choice = choose_announcement(&hand, Cult::Xephis, &mut rng);
            assert!(matches!(choice, None | Some(2)));
        }
        assert_eq!(choose_announcement(&[], Cult::Xephis, &mut rng), None);
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
            choose_production(2, &supply, Some(Cult::Yaboul), Cult::Xephis),
            ItemCounts::new(0, 0, 2)
        );
        assert_eq!(
            choose_production(2, &supply, None, Cult::Xephis),
            ItemCounts::new(2, 0, 0)
        );
    }

    #[test]
    fn conversion_is_always_legal() {
        let mut rng = rand::rng();
        let empty = BoardState::default();
        for _ in 0..20 {
            let choice = choose_conversion(&empty, 1, Some(Cult::Yaboul), &mut rng);
            assert!(matches!(
                choice,
                Some(Conversion::Place {
                    cult: Cult::Yaboul,
                    ..
                })
            ));
            assert!(empty.validate(&choice.unwrap(), 1).is_ok());
        }
    }

    #[test]
    fn conversion_takes_hidden_rival_villages_first() {
        use std::collections::BTreeMap;

        use crate::domain::{CultMarker, SiteId};

        let hidden = CultMarker {
            cult: Cult::Xephis,
            face_up: false,
        };
        let board = BoardState::new(BTreeMap::from([(SiteId(11), hidden)]), Vec::new());
        assert_eq!(
            choose_conversion(&board, 2, Some(Cult::Yaboul), &mut rand::rng()),
            Some(Conversion::Replace { site: SiteId(11) })
        );
    }

    #[test]
    fn production_falls_back_when_the_supply_runs_out() {
        let supply = ItemCounts::new(1, 0, 3);
        assert_eq!(
            choose_production(3, &supply, Some(Cult::Xephis), Cult::Xephis),
            ItemCounts::new(1, 0, 2)
        );
        assert_eq!(
            choose_production(3, &supply, None, Cult::Xephis),
            ItemCounts::new(1, 0, 2)
        );
        assert_eq!(
            choose_production(9, &supply, None, Cult::Xephis),
            ItemCounts::new(1, 0, 3)
        );
        let even = ItemCounts::new(0, 3, 3);
        assert_eq!(
            choose_production(2, &even, None, Cult::Yaboul),
            ItemCounts::new(0, 0, 2)
        );
        assert_eq!(
            choose_production(2, &even, None, Cult::Xephis),
            ItemCounts::new(0, 2, 0)
        );
    }
}
