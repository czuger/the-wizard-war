//! Phase II.1: which site the AI converts.

use rand::Rng;

use crate::domain::ai_strategy::{conversion_priority, preferred_cult};
use crate::domain::board::{SITES, neighbours};
use crate::domain::strong_ai::buyer::best_buyer;
use crate::domain::strong_ai::item_values::ItemValues;
use crate::domain::strong_ai::near_best::pick_near_best;
use crate::domain::strong_ai::outlook::AiOutlook;
use crate::domain::travel::distance;
use crate::domain::{BoardState, Conversion, Cult, Fanatic, Location, MagicItem};

/// Worth, to a rival pawn, of each site that buys within its reach.
const RIVAL_SITE_WORTH: i64 = 800;

/// Steps within which a site is in a rival pawn's reach.
const RIVAL_REACH: usize = 2;

/// Cost of a rebels pawn next to one of the AI's pawns: it may march onto it.
const REBEL_THREAT: i64 = 1_500;

/// Chooses the conversion of phase II.1.
///
/// Every legal conversion is tried on a copy of the board, which is scored by what the AI's
/// stock and loaded pawns can fetch, minus the buyers within its rivals' reach and the
/// rebels next to its pawns, as much as the AI cares about its rivals. Ties go to the
/// beginner's order for the cult the AI holds most items of (castles and monasteries
/// first). The AI picks one of the conversions within its temperament's tolerance.
///
/// # Returns
/// `None` when no conversion is legal.
pub fn choose_conversion<R: Rng>(outlook: &AiOutlook, rng: &mut R) -> Option<Conversion> {
    let cult = favoured_cult(outlook);
    let scored = outlook
        .board
        .legal_conversions(outlook.turn)
        .into_iter()
        .filter_map(|conversion| {
            let after = outlook.board.after(&conversion, outlook.turn).ok()?;
            let priority = f64::from(conversion_priority(&outlook.board, cult, &conversion));
            // The priority only breaks ties: it is worth less than one écu.
            Some((
                conversion,
                board_score(outlook, &after) as f64 - priority / 10.0,
            ))
        })
        .collect();
    pick_near_best(scored, outlook.temperament.tolerance as f64, rng)
}

/// The cult the AI holds most items of, or the one its fanatics are strongest in, or its
/// home cult.
fn favoured_cult(outlook: &AiOutlook) -> Cult {
    let held = |cult: Cult| {
        let item = MagicItem::of_cult(cult);
        let carried = outlook.pawns.iter().filter(|pawn| pawn.item == Some(item));
        outlook.stock.get(item) + carried.count() as i64
    };
    let xephis = held(Cult::Xephis);
    let yaboul = held(Cult::Yaboul);
    if xephis != yaboul {
        return if xephis > yaboul {
            Cult::Xephis
        } else {
            Cult::Yaboul
        };
    }
    let fanatics: Vec<Fanatic> = outlook
        .hand
        .iter()
        .copied()
        .chain(outlook.own_in_hall)
        .collect();
    let home = outlook.temperament.home_cult;
    preferred_cult(&fanatics, home).unwrap_or(home)
}

/// How good `board` is for the AI, in écus.
fn board_score(outlook: &AiOutlook, board: &BoardState) -> i64 {
    let values = ItemValues::build(outlook, board);
    values.stock_value(&outlook.stock) + carried_value(outlook, board)
        - rival_access(outlook, board)
        - rebel_threat(outlook, board)
}

/// What the AI's loaded pawns on the map can fetch.
fn carried_value(outlook: &AiOutlook, board: &BoardState) -> i64 {
    outlook
        .pawns
        .iter()
        .filter(|pawn| pawn.location != Location::City)
        .filter_map(|pawn| {
            let item = pawn.item?;
            best_buyer(outlook, board, item, pawn.location, pawn.kind, &[])
        })
        .map(|buyer| buyer.worth(&outlook.temperament, &outlook.rivals))
        .sum()
}

/// Face-up markers within reach of the rivals' pawns, valued for them.
fn rival_access(outlook: &AiOutlook, board: &BoardState) -> i64 {
    let visible: Vec<Location> = SITES
        .iter()
        .filter(|site| board.marker(site.id).is_some_and(|marker| marker.face_up))
        .map(|site| Location::Site(site.id))
        .collect();
    let in_reach: usize = outlook
        .rivals
        .iter()
        .map(|rival| {
            visible
                .iter()
                .filter(|&&site| distance(rival.location, site).is_some_and(|d| d <= RIVAL_REACH))
                .count()
        })
        .sum();
    (in_reach as f64 * RIVAL_SITE_WORTH as f64 * outlook.temperament.rival_weight) as i64
}

/// Rebels next to the AI's pawns on the map. Rebels only fight where they arrive, so those
/// already on a pawn's site do not threaten it.
fn rebel_threat(outlook: &AiOutlook, board: &BoardState) -> i64 {
    let threats: usize = outlook
        .pawns
        .iter()
        .filter_map(|pawn| pawn.location.site())
        .flat_map(neighbours)
        .map(|next| board.rebels_on(next))
        .sum();
    threats as i64 * REBEL_THREAT
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::domain::strong_ai::outlook::RivalPawn;
    use crate::domain::strong_ai::outlook::fixtures::{outlook, pawn};
    use crate::domain::{CultMarker, PawnKind, SiteId, Wealth};

    #[test]
    fn a_buyer_for_its_loaded_pawn_raises_the_score() {
        let mut outlook = outlook();
        outlook.turn = 3;
        let hidden = CultMarker {
            cult: Cult::Yaboul,
            face_up: false,
        };
        outlook.board = BoardState::new(BTreeMap::from([(SiteId(27), hidden)]), Vec::new());
        let at_27 = Location::Site(SiteId(27));
        outlook.pawns = vec![pawn(1, PawnKind::Peddler, at_27, Some(MagicItem::Xephis))];
        let replaced = outlook
            .board
            .after(&Conversion::Replace { site: SiteId(27) }, 3)
            .unwrap();
        let before = board_score(&outlook, &outlook.board);
        assert_eq!(board_score(&outlook, &replaced) - before, 5_000);
        let choice = choose_conversion(&outlook, &mut rand::rng()).unwrap();
        let chosen = outlook.board.after(&choice, 3).unwrap();
        assert!(board_score(&outlook, &chosen) >= before + 5_000);
    }

    #[test]
    fn hides_buyers_next_to_rival_pawns() {
        let mut outlook = outlook();
        outlook.turn = 3;
        let markers = BTreeMap::from([(SiteId(28), CultMarker::face_up(Cult::Yaboul))]);
        outlook.board = BoardState::new(markers, Vec::new());
        outlook.rivals = vec![RivalPawn {
            location: Location::Site(SiteId(28)),
            kind: PawnKind::Caravan,
            owner_wealth: Wealth::of(0),
        }];
        let choice = choose_conversion(&outlook, &mut rand::rng());
        assert_eq!(choice, Some(Conversion::Flip { site: SiteId(28) }));
    }

    #[test]
    fn conversions_are_legal() {
        let outlook = outlook();
        for _ in 0..10 {
            let choice = choose_conversion(&outlook, &mut rand::rng()).unwrap();
            assert!(outlook.board.validate(&choice, outlook.turn).is_ok());
        }
    }
}
