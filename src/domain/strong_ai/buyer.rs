//! Where a pawn can sell an item, and how long it takes to get there.

use crate::domain::board::SITES;
use crate::domain::market::{buys_without_limit, sale_price};
use crate::domain::strong_ai::outlook::{AiOutlook, RivalPawn};
use crate::domain::travel::distance;
use crate::domain::{AiTemperament, BoardState, Location, MagicItem, PawnKind, SiteId};

/// Chance, per turn of travel, that a village changes or hides its cult before the pawn
/// arrives. Castles and monasteries never change.
pub const FLIP_RISK_PER_TURN: f64 = 0.06;

/// Share of a buyer each rival pawn able to get there first is counted as taking: its load
/// is hidden, so it may carry something else.
const CONTEST_WEIGHT: f64 = 0.5;

/// A site that buys an item, seen from a pawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Buyer {
    /// The buying site.
    pub site: SiteId,
    /// The item it buys.
    pub item: MagicItem,
    /// What it pays, from Table 1.
    pub price: i64,
    /// Turns the pawn needs to get there; 0 when it already stands on it.
    pub turns: i64,
    /// Whether its cult can never change (castles and monasteries).
    pub permanent: bool,
    /// Whether it buys any number of items a turn (monasteries).
    pub unlimited: bool,
}

impl Buyer {
    /// What the sale is worth to an AI: the price, discounted by the risk that the site
    /// stops buying and by the rival pawns that may sell there first, minus the trip, plus
    /// the AI's liking for the item.
    pub fn worth(&self, temperament: &AiTemperament, rivals: &[RivalPawn]) -> i64 {
        let survival = if self.permanent {
            1.0
        } else {
            (1.0 - FLIP_RISK_PER_TURN).powi(self.turns as i32)
        };
        let share = 1.0 / (1.0 + CONTEST_WEIGHT * self.contenders(rivals) as f64);
        let bonus = if temperament.favourite == Some(self.item) {
            temperament.favourite_bonus
        } else {
            0
        };
        (self.price as f64 * survival * share).round() as i64 - temperament.turn_cost * self.turns
            + bonus
    }

    /// Rival pawns on the map that reach this one-item-a-turn site no later than the pawn.
    fn contenders(&self, rivals: &[RivalPawn]) -> usize {
        if self.unlimited {
            return 0;
        }
        let here = Location::Site(self.site);
        rivals
            .iter()
            .filter(|rival| rival.location != Location::City)
            .filter(|rival| {
                travel_turns(rival.kind, rival.location, here).is_some_and(|t| t <= self.turns)
            })
            .count()
    }
}

/// Turns a pawn of `kind` needs to go from `from` to `to`.
///
/// # Returns
/// `None` when `to` cannot be reached.
pub fn travel_turns(kind: PawnKind, from: Location, to: Location) -> Option<i64> {
    let steps = distance(from, to)? as i64;
    let speed = kind.speed() as i64;
    Some((steps + speed - 1) / speed)
}

/// Every site that buys `item` and that a pawn of `kind` on `from` reaches in time to sell.
///
/// # Arguments
/// * `sale_turns` - phases V left in the game, this turn's included if it is still ahead.
pub fn buyers(
    board: &BoardState,
    item: MagicItem,
    from: Location,
    kind: PawnKind,
    sale_turns: i64,
) -> Vec<Buyer> {
    SITES
        .iter()
        .filter_map(|site| {
            let price = sale_price(site, board.marker(site.id), item)?;
            let turns = travel_turns(kind, from, Location::Site(site.id))?;
            (turns.max(1) <= sale_turns).then_some(Buyer {
                site: site.id,
                item,
                price,
                turns,
                permanent: site.kind.is_permanent(),
                unlimited: buys_without_limit(site),
            })
        })
        .collect()
}

/// The buyer worth most to the AI of `outlook` on `board`, leaving out the `excluded` sites.
pub fn best_buyer(
    outlook: &AiOutlook,
    board: &BoardState,
    item: MagicItem,
    from: Location,
    kind: PawnKind,
    excluded: &[SiteId],
) -> Option<Buyer> {
    buyers(board, item, from, kind, outlook.sale_turns)
        .into_iter()
        .filter(|buyer| !excluded.contains(&buyer.site))
        .max_by_key(|buyer| {
            (
                buyer.worth(&outlook.temperament, &outlook.rivals),
                buyer.price,
            )
        })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::domain::strong_ai::outlook::fixtures::outlook;
    use crate::domain::{Archetype, Cult, CultMarker, Wealth};

    fn board(markers: &[(u8, Cult)]) -> BoardState {
        let markers = markers
            .iter()
            .map(|&(site, cult)| (SiteId(site), CultMarker::face_up(cult)))
            .collect::<BTreeMap<_, _>>();
        BoardState::new(markers, Vec::new())
    }

    #[test]
    fn caravans_travel_two_steps_a_turn() {
        let to_16 = Location::Site(SiteId(16));
        assert_eq!(
            travel_turns(PawnKind::Peddler, Location::City, to_16),
            Some(2)
        );
        assert_eq!(
            travel_turns(PawnKind::Caravan, Location::City, to_16),
            Some(1)
        );
        assert_eq!(travel_turns(PawnKind::Caravan, to_16, to_16), Some(0));
    }

    #[test]
    fn far_buyers_must_be_reached_before_the_end() {
        let board = board(&[(27, Cult::Yaboul), (1, Cult::Yaboul)]);
        let mut outlook = outlook();
        let mut far = |sale_turns, excluded: &[SiteId]| {
            outlook.sale_turns = sale_turns;
            best_buyer(
                &outlook,
                &board,
                MagicItem::Yaboul,
                Location::City,
                PawnKind::Caravan,
                excluded,
            )
            .map(|buyer| buyer.site)
        };
        assert_eq!(far(9, &[]), Some(SiteId(1)));
        assert_eq!(far(1, &[]), Some(SiteId(27)));
        assert_eq!(far(0, &[]), None);
        assert_eq!(far(1, &[SiteId(27)]), None);
    }

    #[test]
    fn castles_buy_talismans() {
        let found = buyers(
            &BoardState::default(),
            MagicItem::Toratsa,
            Location::City,
            PawnKind::Peddler,
            9,
        );
        let sites: Vec<SiteId> = found.iter().map(|buyer| buyer.site).collect();
        assert_eq!(sites, vec![SiteId(10), SiteId(21)]);
    }

    fn buyer(permanent: bool, turns: i64) -> Buyer {
        Buyer {
            site: SiteId(16),
            item: MagicItem::Xephis,
            price: 10_000,
            turns,
            permanent,
            unlimited: false,
        }
    }

    #[test]
    fn far_villages_may_stop_buying_before_the_pawn_arrives() {
        let balanced = outlook().temperament;
        let castle = buyer(true, 2).worth(&balanced, &[]);
        let village = buyer(false, 2).worth(&balanced, &[]);
        assert_eq!(castle, 10_000 - 3_000);
        assert!(village < castle);
        assert_eq!(buyer(false, 0).worth(&balanced, &[]), 10_000);
    }

    #[test]
    fn rival_pawns_arriving_first_take_a_share() {
        let balanced = outlook().temperament;
        let near = RivalPawn {
            location: Location::Site(SiteId(16)),
            kind: PawnKind::Peddler,
            owner_wealth: Wealth::of(0),
        };
        let in_city = RivalPawn {
            location: Location::City,
            ..near
        };
        let contested = buyer(true, 1).worth(&balanced, &[near, in_city]);
        assert_eq!(contested, 6_667 - 1_500);
        let monastery = Buyer {
            unlimited: true,
            ..buyer(true, 1)
        };
        assert_eq!(monastery.worth(&balanced, &[near]), 10_000 - 1_500);
    }

    #[test]
    fn the_favourite_item_is_worth_more() {
        let zealot = AiTemperament::new(Archetype::XephisZealot, 0);
        let balanced = outlook().temperament;
        let liked = buyer(true, 0).worth(&zealot, &[]);
        assert_eq!(
            liked - buyer(true, 0).worth(&balanced, &[]),
            zealot.favourite_bonus
        );
    }
}
