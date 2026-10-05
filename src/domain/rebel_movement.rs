//! Phase II.2: each rebels pawn moves one village along a path.

use rand::Rng;
use rand::seq::IndexedRandom;

use crate::domain::board::{neighbours, site_by_id};
use crate::domain::{SiteId, SiteKind};

/// Villages a rebels pawn on `from` may move to. Castles and monasteries stop rebels.
///
/// # Returns
/// The adjacent villages in id order; empty when the pawn cannot move.
pub fn village_destinations(from: SiteId) -> Vec<SiteId> {
    neighbours(from)
        .into_iter()
        .filter(|&id| site_by_id(id).is_some_and(|site| site.kind == SiteKind::Village))
        .collect()
}

/// Picks a random destination for a rebels pawn on `from`.
///
/// # Returns
/// `None` when the pawn cannot move.
pub fn random_destination<R: Rng>(from: SiteId, rng: &mut R) -> Option<SiteId> {
    village_destinations(from).choose(rng).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebels_skip_castles_and_monasteries() {
        assert_eq!(
            village_destinations(SiteId(12)),
            vec![SiteId(3), SiteId(11), SiteId(16)]
        );
        assert_eq!(
            village_destinations(SiteId(14)),
            vec![SiteId(13), SiteId(15)]
        );
    }

    #[test]
    fn random_destination_is_adjacent() {
        let mut rng = rand::rng();
        for _ in 0..20 {
            let to = random_destination(SiteId(23), &mut rng);
            assert!(to.is_some_and(|to| village_destinations(SiteId(23)).contains(&to)));
        }
    }
}
