use crate::db::PawnRecord;
use crate::domain::board::site_by_id;
use crate::domain::market::{buys_without_limit, sale_price};
use crate::domain::{BoardState, ItemCounts, Location, SiteId};

/// Whether a pawn may move in phase V of `turn`: it is not stopped, and it stands on the map
/// or there is an item in its owner's `stock` to load.
pub(crate) fn is_movable(pawn: &PawnRecord, turn: i64, stock: &ItemCounts) -> bool {
    pawn.can_move(turn) && (pawn.location != Location::City || stock.total() > 0)
}

/// The price `pawn` gets for its item where it stands, if it may sell it there now.
///
/// # Arguments
/// * `sold_sites` - sites that have already bought an item this turn.
pub(crate) fn sale_offer(
    pawn: &PawnRecord,
    board: &BoardState,
    sold_sites: &[SiteId],
) -> Option<i64> {
    let item = pawn.item?;
    let site = site_by_id(pawn.location.site()?)?;
    if sold_sites.contains(&site.id) && !buys_without_limit(site) {
        return None;
    }
    sale_price(site, board.marker(site.id), item)
}

/// Whether `from` may pass its item to `to` in phase V.2: two pawns of the same owner on the
/// same site, `from` loaded. When `to` is loaded too, they swap their (different) items.
pub(crate) fn can_transfer(from: &PawnRecord, to: &PawnRecord) -> bool {
    from.id != to.id
        && from.player_id == to.player_id
        && from.location.site().is_some()
        && from.location == to.location
        && from.item.is_some()
        && from.item != to.item
}
