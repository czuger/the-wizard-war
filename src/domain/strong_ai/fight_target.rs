//! Phase II.2: which pawn a rebels pawn attacks when the AI picks.

use crate::domain::{PawnKind, Wealth};

/// A pawn a rebels pawn may attack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FightCandidate {
    /// Id of the pawn.
    pub pawn_id: i64,
    /// Peddler or caravan.
    pub kind: PawnKind,
    /// How rich its owner looks, or `None` for the AI's own pawn.
    pub rival_wealth: Option<Wealth>,
}

/// Chooses the pawn to attack.
///
/// A rival's pawn first: a caravan, which costs more to lose, then the pawn of the richest
/// looking rival. When only the AI's own pawns are there, it risks a peddler first.
///
/// # Returns
/// `None` when there is no candidate.
pub fn choose_fight_target(candidates: &[FightCandidate]) -> Option<i64> {
    candidates
        .iter()
        .max_by_key(|candidate| match candidate.rival_wealth {
            Some(wealth) => (1, candidate.kind == PawnKind::Caravan, wealth.coins()),
            None => (0, candidate.kind == PawnKind::Peddler, 0),
        })
        .map(|candidate| candidate.pawn_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(pawn_id: i64, kind: PawnKind, money: Option<i64>) -> FightCandidate {
        FightCandidate {
            pawn_id,
            kind,
            rival_wealth: money.map(Wealth::of),
        }
    }

    #[test]
    fn attacks_rival_caravans_then_the_richest() {
        let caravan = candidate(1, PawnKind::Caravan, Some(0));
        let rich = candidate(2, PawnKind::Peddler, Some(30_000));
        let own = candidate(3, PawnKind::Caravan, None);
        assert_eq!(choose_fight_target(&[rich, caravan, own]), Some(1));
        assert_eq!(choose_fight_target(&[rich, own]), Some(2));
        let own_peddler = candidate(4, PawnKind::Peddler, None);
        assert_eq!(choose_fight_target(&[own, own_peddler]), Some(4));
        assert_eq!(choose_fight_target(&[]), None);
    }
}
