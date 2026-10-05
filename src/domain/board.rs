//! The map of the county of Arghtal, transcribed from `img/board.jpg` (1055 × 867 pixels).
//!
//! The city has one gate in front of each village of the outskirts (see [`GATE_VILLAGES`]).

use crate::domain::SiteKind::{Castle, Monastery, Village};
use crate::domain::Zone::{Countryside, Mountain, Outskirts};
use crate::domain::{Site, SiteId, SiteKind, Zone};

/// Width of `img/board.jpg`, in pixels.
pub const BOARD_WIDTH: u16 = 1055;

/// Height of `img/board.jpg`, in pixels.
pub const BOARD_HEIGHT: u16 = 867;

/// Centre of the great hall, in the middle of the city, in pixels of `img/board.jpg`.
pub const HALL_POSITION: (u16, u16) = (839, 572);

const fn site(
    id: u8,
    name: &'static str,
    kind: SiteKind,
    zone: Zone,
    position: (u16, u16),
    on_edge: bool,
) -> Site {
    Site {
        id: SiteId(id),
        name,
        kind,
        zone,
        x: position.0,
        y: position.1,
        on_edge,
    }
}

/// Every site of the map: 28 villages, 2 castles and 2 monasteries, ordered by id.
pub static SITES: [Site; 32] = [
    site(1, "Hautecombe", Village, Mountain, (175, 62), true),
    site(2, "Roc-Brumeux", Village, Mountain, (398, 62), false),
    site(3, "Bourg-du-Lac", Village, Mountain, (550, 88), false),
    site(4, "Pierrefeu", Village, Mountain, (668, 40), true),
    site(
        5,
        "Monastère des Cimes",
        Monastery,
        Mountain,
        (925, 42),
        false,
    ),
    site(6, "Combe-Noire", Village, Mountain, (118, 210), true),
    site(7, "Pont-d'Argh", Village, Mountain, (90, 368), false),
    site(8, "Gué-des-Saules", Village, Mountain, (82, 678), true),
    site(9, "Port-Gris", Village, Mountain, (122, 790), false),
    site(
        10,
        "Château de l'Oriflamme",
        Castle,
        Countryside,
        (662, 150),
        false,
    ),
    site(11, "Valbrune", Village, Countryside, (418, 252), false),
    site(12, "Montrouge", Village, Countryside, (575, 215), false),
    site(13, "Clairfontaine", Village, Countryside, (873, 147), false),
    site(14, "Beaupré", Village, Countryside, (955, 188), false),
    site(15, "Rochebelle", Village, Countryside, (1005, 283), false),
    site(16, "Blémont", Village, Countryside, (668, 322), false),
    site(17, "Ormeval", Village, Countryside, (300, 337), false),
    site(18, "Sauvigny", Village, Countryside, (532, 380), false),
    site(19, "Fougerolles", Village, Countryside, (250, 432), false),
    site(20, "Champtier", Village, Countryside, (466, 485), false),
    site(
        21,
        "Château de Bleuvent",
        Castle,
        Countryside,
        (350, 520),
        false,
    ),
    site(22, "Marbois", Village, Countryside, (213, 575), false),
    site(23, "Vauclair", Village, Countryside, (345, 600), false),
    site(24, "Lormel", Village, Countryside, (410, 660), false),
    site(25, "Brumeval", Village, Countryside, (245, 745), false),
    site(
        26,
        "Monastère de la Colline",
        Monastery,
        Countryside,
        (358, 738),
        false,
    ),
    site(27, "Vignelles", Village, Outskirts, (825, 365), false),
    site(28, "Les Granges", Village, Outskirts, (642, 455), false),
    site(29, "Bois-Joli", Village, Outskirts, (1015, 462), false),
    site(30, "Moulinet", Village, Outskirts, (573, 662), false),
    site(31, "Les Tanneries", Village, Outskirts, (630, 780), false),
    site(32, "La Falaise", Village, Outskirts, (957, 775), false),
];

/// Paths joining two sites, each listed once.
static PATHS: [(u8, u8); 44] = [
    (1, 2),
    (2, 3),
    (3, 4),
    (2, 11),
    (3, 12),
    (4, 13),
    (5, 14),
    (6, 7),
    (6, 17),
    (7, 19),
    (7, 22),
    (8, 22),
    (8, 9),
    (9, 25),
    (10, 12),
    (11, 12),
    (11, 16),
    (11, 17),
    (11, 18),
    (11, 20),
    (12, 16),
    (13, 14),
    (14, 15),
    (15, 27),
    (15, 29),
    (16, 27),
    (16, 28),
    (17, 19),
    (18, 20),
    (18, 28),
    (19, 20),
    (19, 23),
    (20, 23),
    (20, 28),
    (20, 30),
    (21, 23),
    (22, 23),
    (23, 24),
    (23, 25),
    (24, 25),
    (24, 30),
    (24, 31),
    (25, 26),
    (31, 32),
];

/// Villages of the outskirts each joined by a path to a gate of the city.
pub static GATE_VILLAGES: [SiteId; 6] = [
    SiteId(27),
    SiteId(28),
    SiteId(29),
    SiteId(30),
    SiteId(31),
    SiteId(32),
];

/// Returns the site with this id, if the board has one.
pub fn site_by_id(id: SiteId) -> Option<&'static Site> {
    SITES.iter().find(|site| site.id == id)
}

/// Returns every path of the map as a pair of site ids.
pub fn paths() -> impl Iterator<Item = (SiteId, SiteId)> {
    PATHS.iter().map(|&(a, b)| (SiteId(a), SiteId(b)))
}

/// Returns the sites a path joins to `id`, in id order.
pub fn neighbours(id: SiteId) -> Vec<SiteId> {
    let mut found: Vec<SiteId> = paths()
        .filter_map(|(a, b)| {
            if a == id {
                Some(b)
            } else if b == id {
                Some(a)
            } else {
                None
            }
        })
        .collect();
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn has_the_sites_of_the_rules() {
        let count = |kind: SiteKind| SITES.iter().filter(|site| site.kind == kind).count();
        assert_eq!(count(Village), 28);
        assert_eq!(count(Castle), 2);
        assert_eq!(count(Monastery), 2);
        for (index, site) in SITES.iter().enumerate() {
            assert_eq!(usize::from(site.id.0), index + 1, "ids follow the array");
            assert!(site.x < BOARD_WIDTH && site.y < BOARD_HEIGHT);
        }
        assert!(SITES.iter().any(|site| site.on_edge));
    }

    #[test]
    fn paths_are_valid_and_unique() {
        let mut seen = BTreeSet::new();
        for (a, b) in paths() {
            assert!(site_by_id(a).is_some() && site_by_id(b).is_some());
            assert_ne!(a, b);
            assert!(
                seen.insert((a.min(b), a.max(b))),
                "duplicate path {a:?}-{b:?}"
            );
        }
    }

    #[test]
    fn the_map_is_connected() {
        let mut reached = BTreeSet::from([SiteId(1)]);
        let mut frontier = vec![SiteId(1)];
        while let Some(id) = frontier.pop() {
            for next in neighbours(id) {
                if reached.insert(next) {
                    frontier.push(next);
                }
            }
        }
        assert_eq!(reached.len(), SITES.len());
    }

    #[test]
    fn gates_open_on_the_outskirts() {
        for id in GATE_VILLAGES {
            assert_eq!(site_by_id(id).map(|site| site.zone), Some(Outskirts));
        }
    }

    #[test]
    fn neighbours_go_both_ways() {
        assert_eq!(neighbours(SiteId(10)), vec![SiteId(12)]);
        assert!(neighbours(SiteId(12)).contains(&SiteId(10)));
        assert!(site_by_id(SiteId(0)).is_none());
    }
}
