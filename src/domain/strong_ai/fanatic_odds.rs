//! Chances of each Dominant Fanatic (phase IV), from what an alchemist can know.

use crate::domain::{Cult, Fanatic};

/// Probability of each outcome of phase IV.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FanaticOdds {
    /// Xéphis dominates.
    pub xephis: f64,
    /// Yaboul dominates.
    pub yaboul: f64,
    /// Tie or empty hall: the fanatics do not raid.
    pub none: f64,
}

impl FanaticOdds {
    /// Computes the exact odds when the hall holds the `known` pawns plus `drawn` pawns taken
    /// at random among the `unseen` ones.
    ///
    /// Every subset of `drawn` unseen pawns is equally likely; they are counted by kind
    /// (cult and strength), so the work stays small.
    pub fn compute(known: &[Fanatic], unseen: &[Fanatic], drawn: usize) -> FanaticOdds {
        let known_lead: i64 = known.iter().map(signed_strength).sum();
        let mut kinds: Vec<(i64, usize)> = Vec::new();
        for fanatic in unseen {
            let lead = signed_strength(fanatic);
            match kinds.iter_mut().find(|(kind, _)| *kind == lead) {
                Some((_, count)) => *count += 1,
                None => kinds.push((lead, 1)),
            }
        }
        let mut tally = Tally::default();
        tally.add_draws(&kinds, drawn.min(unseen.len()), known_lead, 1.0);
        tally.odds()
    }

    /// The probability that `dominant` is the outcome; `None` stands for no raid.
    pub fn of(&self, dominant: Option<Cult>) -> f64 {
        match dominant {
            Some(Cult::Xephis) => self.xephis,
            Some(Cult::Yaboul) => self.yaboul,
            None => self.none,
        }
    }
}

/// Weighted count of the hall's outcomes.
#[derive(Debug, Default)]
struct Tally {
    xephis: f64,
    yaboul: f64,
    none: f64,
}

impl Tally {
    /// Adds every way to draw `left` pawns from `kinds` (lead per pawn, pawns of that kind).
    fn add_draws(&mut self, kinds: &[(i64, usize)], left: usize, lead: i64, weight: f64) {
        let Some((&(kind_lead, count), rest)) = kinds.split_first() else {
            if left == 0 {
                match lead.signum() {
                    1 => self.xephis += weight,
                    -1 => self.yaboul += weight,
                    _ => self.none += weight,
                }
            }
            return;
        };
        for taken in 0..=count.min(left) {
            let ways = binomial(count, taken);
            let new_lead = lead + kind_lead * taken as i64;
            self.add_draws(rest, left - taken, new_lead, weight * ways);
        }
    }

    fn odds(&self) -> FanaticOdds {
        let total = self.xephis + self.yaboul + self.none;
        if total <= 0.0 {
            return FanaticOdds {
                xephis: 0.0,
                yaboul: 0.0,
                none: 1.0,
            };
        }
        FanaticOdds {
            xephis: self.xephis / total,
            yaboul: self.yaboul / total,
            none: self.none / total,
        }
    }
}

/// Strength counted positive for Xéphis, negative for Yaboul.
fn signed_strength(fanatic: &Fanatic) -> i64 {
    match fanatic.cult {
        Cult::Xephis => fanatic.strength,
        Cult::Yaboul => -fanatic.strength,
    }
}

fn binomial(n: usize, k: usize) -> f64 {
    (0..k).fold(1.0, |acc, i| acc * (n - i) as f64 / (i + 1) as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::FanaticPool;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn known_pawns_alone_decide() {
        let odds = FanaticOdds::compute(&[Fanatic::new(Cult::Xephis, 3)], &[], 2);
        assert_eq!(odds.of(Some(Cult::Xephis)), 1.0);
        let empty = FanaticOdds::compute(&[], &[Fanatic::new(Cult::Yaboul, 1)], 0);
        assert_eq!(empty.of(None), 1.0);
    }

    #[test]
    fn a_strong_pawn_beats_one_weak_unseen_pawn() {
        let odds = FanaticOdds::compute(
            &[Fanatic::new(Cult::Xephis, 3)],
            &[Fanatic::new(Cult::Yaboul, 1)],
            1,
        );
        assert_eq!(odds.xephis, 1.0);
    }

    #[test]
    fn odds_count_every_draw() {
        let unseen = [
            Fanatic::new(Cult::Yaboul, 2),
            Fanatic::new(Cult::Yaboul, 1),
            Fanatic::new(Cult::Xephis, 1),
        ];
        let odds = FanaticOdds::compute(&[Fanatic::new(Cult::Xephis, 1)], &unseen, 1);
        assert!(close(odds.yaboul, 1.0 / 3.0));
        assert!(close(odds.none, 1.0 / 3.0));
        assert!(close(odds.xephis, 1.0 / 3.0));
    }

    #[test]
    fn odds_sum_to_one_on_the_full_pool() {
        let pool = FanaticPool::full();
        let odds = FanaticOdds::compute(&[], pool.tokens(), 5);
        assert!(close(odds.xephis + odds.yaboul + odds.none, 1.0));
        assert!(odds.xephis > odds.yaboul, "Xéphis has more fanatics");
    }
}
