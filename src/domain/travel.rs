//! Phase V: how peddlers and caravans move between the city and the sites of the map.
//!
//! The city is one more stop of the travel graph, joined to each gate village: leaving
//! the city through a gate and reaching a gate village is one step, and so is coming back.
//! A pawn that reaches a gate goes back inside at once, so it never crosses the city.

use std::collections::{BTreeMap, VecDeque};

use crate::domain::board::{GATE_VILLAGES, neighbours};
use crate::domain::{Location, PawnKind};

/// Stops one step away from `from`.
pub fn travel_neighbours(from: Location) -> Vec<Location> {
    match from {
        Location::City => GATE_VILLAGES.iter().copied().map(Location::Site).collect(),
        Location::Site(site) => {
            let mut found: Vec<Location> =
                neighbours(site).into_iter().map(Location::Site).collect();
            if GATE_VILLAGES.contains(&site) {
                found.push(Location::City);
            }
            found
        }
    }
}

/// Distance in steps from `from` to every stop reachable from it.
fn distances(from: Location) -> BTreeMap<Location, (usize, Location)> {
    let mut reached = BTreeMap::from([(from, (0, from))]);
    let mut queue = VecDeque::from([from]);
    while let Some(stop) = queue.pop_front() {
        if stop == Location::City && stop != from {
            continue;
        }
        let distance = reached.get(&stop).map_or(0, |(distance, _)| *distance);
        for next in travel_neighbours(stop) {
            if let std::collections::btree_map::Entry::Vacant(entry) = reached.entry(next) {
                entry.insert((distance + 1, stop));
                queue.push_back(next);
            }
        }
    }
    reached
}

/// Every stop a pawn of `kind` on `from` may end its move on this turn, `from` excluded.
pub fn destinations(kind: PawnKind, from: Location) -> Vec<Location> {
    distances(from)
        .into_iter()
        .filter(|(stop, (distance, _))| *stop != from && *distance <= kind.speed())
        .map(|(stop, _)| stop)
        .collect()
}

/// A shortest route from `from` to `to`, both included.
///
/// # Returns
/// `None` when `to` cannot be reached.
pub fn shortest_path(from: Location, to: Location) -> Option<Vec<Location>> {
    let reached = distances(from);
    reached.get(&to)?;
    let mut path = vec![to];
    let mut stop = to;
    while stop != from {
        stop = reached.get(&stop)?.1;
        path.push(stop);
    }
    path.reverse();
    Some(path)
}

/// Number of steps from `from` to `to`, if `to` can be reached.
pub fn distance(from: Location, to: Location) -> Option<usize> {
    distances(from).get(&to).map(|(distance, _)| *distance)
}

/// The farthest stop a pawn of `kind` reaches this turn on a shortest route to `to`.
///
/// # Returns
/// `from` when `to` cannot be reached.
pub fn step_toward(kind: PawnKind, from: Location, to: Location) -> Location {
    shortest_path(from, to)
        .and_then(|path| path.get(kind.speed().min(path.len() - 1)).copied())
        .unwrap_or(from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SiteId;

    const fn site(id: u8) -> Location {
        Location::Site(SiteId(id))
    }

    #[test]
    fn peddlers_leave_through_a_gate() {
        let from_city = destinations(PawnKind::Peddler, Location::City);
        assert_eq!(from_city, (27..=32).map(site).collect::<Vec<_>>());
        assert!(destinations(PawnKind::Peddler, site(27)).contains(&Location::City));
        assert!(!destinations(PawnKind::Peddler, site(16)).contains(&Location::City));
    }

    #[test]
    fn caravans_go_two_steps_and_never_cross_the_city() {
        let from_16 = destinations(PawnKind::Caravan, site(16));
        assert!(from_16.contains(&Location::City), "16 → 27 → city");
        assert!(from_16.contains(&site(15)), "16 → 27 → 15");
        assert!(!from_16.contains(&site(16)));
        let from_32 = destinations(PawnKind::Caravan, site(32));
        assert_eq!(
            from_32,
            vec![Location::City, site(24), site(31)],
            "other gate villages are only reached through the city"
        );
    }

    #[test]
    fn routes_and_steps() {
        let path = shortest_path(Location::City, site(16)).unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(distance(Location::City, site(16)), Some(2));
        assert!(matches!(
            step_toward(PawnKind::Peddler, Location::City, site(16)),
            Location::Site(SiteId(27 | 28))
        ));
        assert_eq!(
            step_toward(PawnKind::Caravan, Location::City, site(16)),
            site(16)
        );
        assert_eq!(step_toward(PawnKind::Caravan, site(16), site(16)), site(16));
        assert_eq!(
            step_toward(PawnKind::Caravan, site(1), Location::City),
            site(11)
        );
    }
}
