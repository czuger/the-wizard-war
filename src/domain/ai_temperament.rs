//! The hidden playing style of a computer-controlled alchemist.
//!
//! Every AI of a game draws a different [`Archetype`] at creation, plus a seed that varies
//! its weights a little, so that the AIs neither play alike nor play the same way from one
//! game to the next. Nothing of it is shown to the human player.

use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

use crate::domain::{Cult, MagicItem};

/// Largest relative change the seed makes to each weight of an archetype.
const JITTER: f64 = 0.15;

/// A family of playing styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Archetype {
    /// The plain evaluating strategy, with no leaning; the human player's seat and the
    /// AIs of games created before temperaments existed.
    Balanced,
    /// Devoted to Xéphis: makes and sells its grimoires, sends fanatics readily.
    XephisZealot,
    /// Devoted to Yaboul: makes and sells its pentacles, sends fanatics readily.
    YaboulZealot,
    /// Prudent trader: favours Toratsa talismans, which the fanatics never raid and the
    /// castles always buy.
    Merchant,
    /// Goes wherever the best price is, far away if need be, and takes chances.
    Opportunist,
    /// Hinders its rivals: hides the buyers near their pawns and sends rebels at them.
    Brigand,
}

impl Archetype {
    /// The archetypes dealt to the computer-controlled alchemists, one each.
    pub const DRAWN: [Archetype; 5] = [
        Archetype::XephisZealot,
        Archetype::YaboulZealot,
        Archetype::Merchant,
        Archetype::Opportunist,
        Archetype::Brigand,
    ];

    /// Every archetype.
    const ALL: [Archetype; 6] = [
        Archetype::Balanced,
        Archetype::XephisZealot,
        Archetype::YaboulZealot,
        Archetype::Merchant,
        Archetype::Opportunist,
        Archetype::Brigand,
    ];

    /// Returns the value stored in the database.
    pub fn key(self) -> &'static str {
        match self {
            Archetype::Balanced => "balanced",
            Archetype::XephisZealot => "xephis_zealot",
            Archetype::YaboulZealot => "yaboul_zealot",
            Archetype::Merchant => "merchant",
            Archetype::Opportunist => "opportunist",
            Archetype::Brigand => "brigand",
        }
    }

    /// Parses a value made by [`Archetype::key`].
    ///
    /// # Returns
    /// `None` when the value is not a known archetype.
    pub fn from_key(value: &str) -> Option<Archetype> {
        Archetype::ALL
            .into_iter()
            .find(|archetype| archetype.key() == value)
    }
}

/// The weights an AI's decisions use.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AiTemperament {
    /// The family of the style.
    pub archetype: Archetype,
    /// The cult the AI leans to when nothing else decides.
    pub home_cult: Cult,
    /// The item the AI likes to make and carry, if any.
    pub favourite: Option<MagicItem>,
    /// What the favourite item is worth more to the AI, in écus.
    pub favourite_bonus: i64,
    /// What a turn of travel costs a pawn, in écus.
    pub turn_cost: i64,
    /// Odds of dominating above which the AI sends a fanatic.
    pub send_odds: f64,
    /// Écus given up per écu of standard deviation of the outcome of a production.
    pub risk_aversion: f64,
    /// Weight of what the AI's moves do to its rivals.
    pub rival_weight: f64,
    /// Share of the usual city expense the AI pays.
    pub expense_factor: f64,
    /// Écus within which an option counts as good as the best one.
    pub tolerance: i64,
}

impl AiTemperament {
    /// The weights of `archetype`, varied by `seed`. [`Archetype::Balanced`] is never varied,
    /// and leans to Xéphis for a seed of 0, as before temperaments existed.
    pub fn new(archetype: Archetype, seed: i64) -> AiTemperament {
        // Reinterpreting the stored bits as a seed is intended.
        let mut rng = StdRng::seed_from_u64(seed as u64);
        let seeded_cult = if seed.rem_euclid(2) == 0 {
            Cult::Xephis
        } else {
            Cult::Yaboul
        };
        let base = AiTemperament::base(archetype, seeded_cult);
        if archetype == Archetype::Balanced {
            return base;
        }
        let mut vary = |value: f64| value * rng.random_range(1.0 - JITTER..=1.0 + JITTER);
        AiTemperament {
            favourite_bonus: vary(base.favourite_bonus as f64) as i64,
            turn_cost: vary(base.turn_cost as f64) as i64,
            send_odds: vary(base.send_odds).clamp(0.3, 0.9),
            risk_aversion: vary(base.risk_aversion),
            rival_weight: vary(base.rival_weight),
            expense_factor: vary(base.expense_factor),
            tolerance: vary(base.tolerance as f64) as i64,
            ..base
        }
    }

    /// The unvaried weights of `archetype`.
    fn base(archetype: Archetype, seeded_cult: Cult) -> AiTemperament {
        let balanced = AiTemperament {
            archetype,
            home_cult: seeded_cult,
            favourite: None,
            favourite_bonus: 0,
            turn_cost: 1_500,
            send_odds: 0.6,
            risk_aversion: 0.0,
            rival_weight: 1.0,
            expense_factor: 1.0,
            tolerance: 0,
        };
        let zealot = |cult: Cult| AiTemperament {
            home_cult: cult,
            favourite: Some(MagicItem::of_cult(cult)),
            favourite_bonus: 2_000,
            send_odds: 0.45,
            tolerance: 600,
            ..balanced
        };
        match archetype {
            Archetype::Balanced => balanced,
            Archetype::XephisZealot => zealot(Cult::Xephis),
            Archetype::YaboulZealot => zealot(Cult::Yaboul),
            Archetype::Merchant => AiTemperament {
                favourite: Some(MagicItem::Toratsa),
                favourite_bonus: 2_500,
                send_odds: 0.75,
                risk_aversion: 0.5,
                rival_weight: 0.8,
                expense_factor: 1.5,
                tolerance: 500,
                ..balanced
            },
            Archetype::Opportunist => AiTemperament {
                turn_cost: 1_000,
                send_odds: 0.55,
                tolerance: 1_000,
                ..balanced
            },
            Archetype::Brigand => AiTemperament {
                turn_cost: 1_700,
                risk_aversion: 0.2,
                rival_weight: 1.6,
                expense_factor: 0.6,
                tolerance: 700,
                ..balanced
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip() {
        for archetype in Archetype::ALL {
            assert_eq!(Archetype::from_key(archetype.key()), Some(archetype));
        }
        assert_eq!(Archetype::from_key("sage"), None);
    }

    #[test]
    fn balanced_keeps_the_plain_weights() {
        let balanced = AiTemperament::new(Archetype::Balanced, 0);
        assert_eq!(balanced.home_cult, Cult::Xephis);
        assert_eq!(balanced.turn_cost, 1_500);
        assert_eq!(balanced.tolerance, 0);
        assert_eq!(balanced.favourite, None);
    }

    #[test]
    fn seeds_vary_the_weights_within_bounds() {
        for seed in [-7, 0, 1, 42, i64::MAX] {
            for archetype in Archetype::DRAWN {
                let varied = AiTemperament::new(archetype, seed);
                let base = AiTemperament::base(archetype, varied.home_cult);
                let near =
                    |value: f64, plain: f64| (value - plain).abs() <= plain.abs() * JITTER + 1.0;
                assert!(near(varied.turn_cost as f64, base.turn_cost as f64));
                assert!(near(varied.rival_weight, base.rival_weight));
                assert!(near(varied.tolerance as f64, base.tolerance as f64));
                assert!((0.3..=0.9).contains(&varied.send_odds));
                assert_eq!(varied.favourite, base.favourite);
            }
        }
        let zealot = AiTemperament::new(Archetype::YaboulZealot, 0);
        assert_eq!(zealot.home_cult, Cult::Yaboul);
        assert_eq!(zealot.favourite, Some(MagicItem::Yaboul));
        assert_eq!(
            AiTemperament::new(Archetype::Merchant, 3),
            AiTemperament::new(Archetype::Merchant, 3)
        );
    }
}
