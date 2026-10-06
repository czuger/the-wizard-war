//! Phase VI: whether the AI sells an item in the city, and which.

use crate::domain::MagicItem;
use crate::domain::market::city_price;
use crate::domain::strong_ai::outlook::AiOutlook;

/// Chooses the item to sell in the city.
///
/// An item is sold when the city pays more than it is worth in the stock: no pawn will take
/// it out before the end, or a sale on the map would barely pay more once the trip is
/// counted. Among those, the AI sells the one it gains most on.
///
/// # Returns
/// `None` to pass.
pub fn choose_city_sale(outlook: &AiOutlook) -> Option<MagicItem> {
    let values = outlook.item_values();
    let stock = outlook.stock;
    let worth = values.stock_value(&stock);
    MagicItem::ALL
        .into_iter()
        .filter(|&item| stock.get(item) > 0)
        .filter_map(|item| {
            let kept = worth - values.stock_value(&stock.with(item, stock.get(item) - 1));
            let gain = city_price(item) - kept;
            (gain > 0).then_some((item, gain))
        })
        .max_by_key(|(_, gain)| *gain)
        .map(|(item, _)| item)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::domain::strong_ai::outlook::fixtures::{outlook, pawn};
    use crate::domain::{BoardState, Cult, CultMarker, ItemCounts, Location, PawnKind, SiteId};

    #[test]
    fn keeps_what_its_pawns_sell_better_and_sells_the_rest() {
        let mut outlook = outlook();
        let markers = BTreeMap::from([(SiteId(1), CultMarker::face_up(Cult::Yaboul))]);
        outlook.board = BoardState::new(markers, Vec::new());
        outlook.pawns = vec![pawn(1, PawnKind::Caravan, Location::City, None)];
        outlook.stock = ItemCounts::new(0, 0, 1);
        assert_eq!(choose_city_sale(&outlook), None);
        outlook.stock = ItemCounts::new(0, 1, 1);
        assert_eq!(choose_city_sale(&outlook), Some(MagicItem::Xephis));
    }

    #[test]
    fn sells_everything_once_the_map_is_closed() {
        let mut outlook = outlook();
        outlook.sale_turns = 0;
        outlook.stock = ItemCounts::new(0, 0, 1);
        assert_eq!(choose_city_sale(&outlook), Some(MagicItem::Yaboul));
        outlook.stock = ItemCounts::default();
        assert_eq!(choose_city_sale(&outlook), None);
    }
}
