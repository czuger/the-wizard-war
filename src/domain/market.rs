//! Sale prices of magic items (Table 1) and the city's demand (Table 2).

use crate::domain::{CultMarker, MagicItem, Site, SiteKind, Zone};

/// Table 1: price a site of `zone` pays for `item`, in écus, if items of that kind sell there.
pub fn zone_price(item: MagicItem, zone: Zone) -> Option<i64> {
    match (item, zone) {
        (MagicItem::Toratsa, Zone::Countryside) => Some(12_500),
        (MagicItem::Toratsa, _) => None,
        (MagicItem::Xephis, Zone::Outskirts) => Some(5_000),
        (MagicItem::Xephis, Zone::Countryside) => Some(7_000),
        (MagicItem::Xephis, Zone::Mountain) => Some(13_000),
        (MagicItem::Yaboul, Zone::Outskirts) => Some(5_000),
        (MagicItem::Yaboul, Zone::Countryside) => Some(8_000),
        (MagicItem::Yaboul, Zone::Mountain) => Some(16_000),
    }
}

/// Table 1: price the city pays for `item` in phase VI, in écus.
pub fn city_price(item: MagicItem) -> i64 {
    match item {
        MagicItem::Toratsa => 4_000,
        MagicItem::Xephis => 4_500,
        MagicItem::Yaboul => 3_500,
    }
}

/// Phase V.2: what `site`, holding `marker`, pays for `item`.
///
/// A site buys the items of the cult its face-up marker shows; a face-down marker hides the
/// site's cult, so nothing sells there. Castles also buy Toratsa talismans, whatever their
/// marker.
///
/// # Returns
/// `None` when the site does not buy this item.
pub fn sale_price(site: &Site, marker: Option<CultMarker>, item: MagicItem) -> Option<i64> {
    let buys = match item.cult() {
        None => site.kind == SiteKind::Castle,
        Some(cult) => marker.is_some_and(|marker| marker.face_up && marker.cult == cult),
    };
    if buys {
        zone_price(item, site.zone)
    } else {
        None
    }
}

/// Whether `site` buys any number of items in one turn; other sites buy one item per turn.
pub fn buys_without_limit(site: &Site) -> bool {
    site.kind == SiteKind::Monastery
}

/// Table 2 rows: first and last number of laboratories on the board for each row.
pub const CITY_QUOTA_LABORATORY_BANDS: [(i64, i64); 4] = [(2, 4), (5, 9), (10, 14), (15, 21)];

/// Table 2 columns: lowest collective expense of each column, in écus.
pub const CITY_QUOTA_EXPENSE_STEPS: [i64; 6] = [0, 700, 2_000, 4_000, 6_000, 8_000];

/// Table 2: items the city buys, by laboratory row and expense column.
pub const CITY_QUOTA_TABLE: [[i64; 6]; 4] = [
    [0, 1, 3, 3, 3, 3],
    [0, 0, 3, 6, 6, 6],
    [0, 0, 0, 6, 9, 9],
    [0, 0, 0, 0, 9, 12],
];

/// Table 2 row for `laboratories` on the board; fewer than 2 reads the first row.
pub fn city_quota_row(laboratories: i64) -> usize {
    CITY_QUOTA_LABORATORY_BANDS
        .iter()
        .position(|&(_, last)| laboratories <= last)
        .unwrap_or(CITY_QUOTA_LABORATORY_BANDS.len() - 1)
}

/// Table 2 column for a collective expense of `collective_expense` écus.
fn city_quota_column(collective_expense: i64) -> usize {
    CITY_QUOTA_EXPENSE_STEPS
        .iter()
        .rposition(|&step| collective_expense >= step)
        .unwrap_or(0)
}

/// Table 2: how many items all the alchemists together may sell in the city this turn.
///
/// # Arguments
/// * `laboratories` - laboratories on the board; fewer than 2 reads the first row.
/// * `collective_expense` - écus paid into the envelope this turn.
pub fn city_sale_quota(laboratories: i64, collective_expense: i64) -> i64 {
    CITY_QUOTA_TABLE[city_quota_row(laboratories)][city_quota_column(collective_expense)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::board::site_by_id;
    use crate::domain::{Cult, SiteId};

    fn site(id: u8) -> &'static Site {
        site_by_id(SiteId(id)).unwrap()
    }

    #[test]
    fn villages_buy_items_of_their_visible_cult() {
        let village = site(1);
        let xephis = Some(CultMarker::face_up(Cult::Xephis));
        assert_eq!(sale_price(village, xephis, MagicItem::Xephis), Some(13_000));
        assert_eq!(sale_price(village, xephis, MagicItem::Yaboul), None);
        assert_eq!(sale_price(village, xephis, MagicItem::Toratsa), None);
        let hidden = xephis.map(CultMarker::flipped);
        assert_eq!(sale_price(village, hidden, MagicItem::Xephis), None);
        assert_eq!(sale_price(village, None, MagicItem::Xephis), None);
    }

    #[test]
    fn castles_always_buy_talismans() {
        let castle = site(10);
        assert_eq!(sale_price(castle, None, MagicItem::Toratsa), Some(12_500));
        let yaboul = Some(CultMarker::face_up(Cult::Yaboul));
        assert_eq!(sale_price(castle, yaboul, MagicItem::Yaboul), Some(8_000));
        assert_eq!(sale_price(site(5), yaboul, MagicItem::Toratsa), None);
        assert!(buys_without_limit(site(5)));
        assert!(!buys_without_limit(castle));
    }

    #[test]
    fn city_quota_follows_table_two() {
        assert_eq!(city_sale_quota(1, 700), 1);
        assert_eq!(city_sale_quota(6, 699), 0);
        assert_eq!(city_sale_quota(6, 2_000), 3);
        assert_eq!(city_sale_quota(12, 6_000), 9);
        assert_eq!(city_sale_quota(21, 8_000), 12);
        assert_eq!(city_sale_quota(15, 7_999), 9);
        assert_eq!(city_sale_quota(1, 0), 0);
        assert_eq!(city_sale_quota(40, 9_000), 12);
    }
}
