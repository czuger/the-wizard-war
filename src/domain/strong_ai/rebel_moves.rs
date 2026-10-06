//! Phase II.2: where the first player sends each rebels pawn.

use rand::Rng;
use rand::seq::IndexedRandom;

use crate::domain::board::neighbours;
use crate::domain::rebel_movement::village_destinations;
use crate::domain::strong_ai::outlook::AiOutlook;
use crate::domain::{Location, SiteId};

/// A rebels pawn of the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rebel {
    /// Id of the pawn.
    pub id: i64,
    /// Village it stands on.
    pub site: SiteId,
}

/// Chooses where each rebels pawn goes.
///
/// A rebels pawn marches onto rival pawns when it can, since the AI then picks whom it
/// attacks; it never marches onto the AI's own pawns if another village is free of them.
/// Otherwise it heads where rival pawns are near and the AI's pawns are not.
///
/// # Returns
/// One `(rebel id, village)` per pawn able to move.
pub fn choose_rebel_moves<R: Rng>(
    outlook: &AiOutlook,
    rebels: &[Rebel],
    rng: &mut R,
) -> Vec<(i64, SiteId)> {
    rebels
        .iter()
        .filter_map(|rebel| {
            let options = village_destinations(rebel.site);
            let best = options.iter().map(|&site| score(outlook, site)).max()?;
            let candidates: Vec<SiteId> = options
                .into_iter()
                .filter(|&site| score(outlook, site) == best)
                .collect();
            candidates.choose(rng).map(|&site| (rebel.id, site))
        })
        .collect()
}

/// How good it is for the AI to send a rebels pawn to `site`.
fn score(outlook: &AiOutlook, site: SiteId) -> i64 {
    let here = Location::Site(site);
    let near = |count: fn(&AiOutlook, Location) -> usize| -> i64 {
        neighbours(site)
            .into_iter()
            .map(|next| count(outlook, Location::Site(next)) as i64)
            .sum()
    };
    let own_here = outlook.own_pawns_on(here) as i64;
    let rivals_here = outlook.rival_pawns_on(here) as i64;
    -100 * own_here + 10 * rivals_here + 2 * near(AiOutlook::rival_pawns_on)
        - 3 * near(AiOutlook::own_pawns_on)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::strong_ai::outlook::RivalPawn;
    use crate::domain::strong_ai::outlook::fixtures::{outlook, pawn};
    use crate::domain::{PawnKind, Wealth};

    #[test]
    fn marches_on_rivals_and_spares_its_own_pawns() {
        let mut outlook = outlook();
        outlook.pawns = vec![pawn(1, PawnKind::Peddler, Location::Site(SiteId(3)), None)];
        outlook.rivals = vec![RivalPawn {
            location: Location::Site(SiteId(16)),
            kind: PawnKind::Peddler,
            owner_wealth: Wealth::of(0),
        }];
        let rebel = Rebel {
            id: 7,
            site: SiteId(12),
        };
        let moves = choose_rebel_moves(&outlook, &[rebel], &mut rand::rng());
        assert_eq!(moves, vec![(7, SiteId(16))]);
        outlook.rivals.clear();
        for _ in 0..20 {
            let moves = choose_rebel_moves(&outlook, &[rebel], &mut rand::rng());
            assert_ne!(moves, vec![(7, SiteId(3))]);
        }
    }
}
