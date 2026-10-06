//! Phase VII: what the AI buys from, and sells back to, the bank.
//!
//! Only écus count at the end, so everything bought must pay for itself in the turns left.

use crate::domain::rules::{BANK_CLOSED_TURNS, LABORATORY_PRICE};
use crate::domain::strong_ai::outlook::AiOutlook;
use crate::domain::{Holdings, PawnKind, Purchase};

/// Écus the AI keeps after buying.
const RESERVE: i64 = 1_000;

/// What one more item a turn is expected to bring, in écus.
const LABORATORY_ITEM_WORTH: i64 = 4_500;

/// What a laboratory must be expected to bring, in percent of its price.
const LABORATORY_MARGIN_PERCENT: i64 = 125;

/// Turns left after this one a pawn needs to pay for itself.
fn turns_to_pay_back(kind: PawnKind) -> i64 {
    match kind {
        PawnKind::Peddler => 2,
        PawnKind::Caravan => 3,
    }
}

/// Chooses what the AI buys and sells back in phase VII.
///
/// It buys a pawn while it has fewer than the items it will have to carry (its stock and
/// its next production, at most two per laboratory), a caravan when it can afford one, and
/// only if the pawn still has time to pay for itself. It buys a laboratory only if the
/// items it makes in the turns left are worth its price with a margin, and only with
/// enough pawns to carry them. On the last turn the bank buys pawns back, it sells those
/// that will have nothing to carry. It always keeps a reserve.
pub fn choose_purchase(outlook: &AiOutlook, holdings: &Holdings) -> Purchase {
    let mut purchase = Purchase::default();
    if holdings.bank_open && outlook.turns_after() == BANK_CLOSED_TURNS {
        sell_idle_pawns(outlook, holdings, &mut purchase);
    }
    let mut money = holdings.money;
    if let Some(kind) = pawn_to_buy(outlook, holdings, money) {
        money -= kind.price();
        match kind {
            PawnKind::Peddler => purchase.peddlers = 1,
            PawnKind::Caravan => purchase.caravans = 1,
        }
    }
    let pawns = holdings.peddlers + holdings.caravans + purchase.peddlers + purchase.caravans;
    let items_made = (outlook.turns_after() - 1).max(0);
    let laboratory_pays =
        items_made * LABORATORY_ITEM_WORTH * 100 >= LABORATORY_PRICE * LABORATORY_MARGIN_PERCENT;
    if laboratory_pays
        && pawns >= outlook.laboratories
        && holdings.laboratories_left > 0
        && money >= LABORATORY_PRICE + RESERVE
    {
        purchase.laboratories = 1;
    }
    if purchase.check(holdings).is_ok() {
        purchase
    } else {
        Purchase::default()
    }
}

/// The pawn the AI needs and can afford, if any.
fn pawn_to_buy(outlook: &AiOutlook, holdings: &Holdings, money: i64) -> Option<PawnKind> {
    let pawns = holdings.peddlers + holdings.caravans;
    let to_carry = (outlook.stock.total() + outlook.laboratories).min(2 * outlook.laboratories);
    if pawns >= to_carry {
        return None;
    }
    [PawnKind::Caravan, PawnKind::Peddler]
        .into_iter()
        .find(|&kind| {
            let owned = match kind {
                PawnKind::Peddler => holdings.peddlers,
                PawnKind::Caravan => holdings.caravans,
            };
            owned < kind.max_owned() as i64
                && outlook.turns_after() >= turns_to_pay_back(kind)
                && money >= kind.price() + RESERVE
        })
}

/// Sells back the pawns in the city that the last turns will not need, peddlers first.
fn sell_idle_pawns(outlook: &AiOutlook, holdings: &Holdings, purchase: &mut Purchase) {
    let needed = outlook.stock.total() + outlook.laboratories * BANK_CLOSED_TURNS;
    let mut idle = (holdings.peddlers + holdings.caravans - needed).max(0);
    purchase.sold_peddlers = idle.min(holdings.peddlers_in_city);
    idle -= purchase.sold_peddlers;
    purchase.sold_caravans = idle.min(holdings.caravans_in_city);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ItemCounts;
    use crate::domain::strong_ai::outlook::fixtures::outlook;

    fn holdings(money: i64, peddlers: i64, caravans: i64) -> Holdings {
        Holdings {
            money,
            peddlers,
            caravans,
            peddlers_in_city: peddlers,
            caravans_in_city: caravans,
            laboratories_left: 15,
            bank_open: true,
        }
    }

    #[test]
    fn buys_carriers_then_laboratories() {
        let outlook = outlook();
        assert_eq!(
            choose_purchase(&outlook, &holdings(5_000, 0, 0)).peddlers,
            1
        );
        assert_eq!(
            choose_purchase(&outlook, &holdings(9_000, 0, 0)).caravans,
            1
        );
        let equipped = choose_purchase(&outlook, &holdings(20_000, 1, 0));
        assert_eq!(equipped.laboratories, 1);
        assert_eq!(equipped.cost(), LABORATORY_PRICE);
    }

    #[test]
    fn buys_nothing_that_cannot_pay_back() {
        let mut outlook = outlook();
        outlook.turn = outlook.max_turns - 1;
        assert!(choose_purchase(&outlook, &holdings(50_000, 0, 0)).is_empty());
        outlook.turn = outlook.max_turns - 3;
        let late = choose_purchase(&outlook, &holdings(50_000, 1, 0));
        assert_eq!(
            late.laboratories, 0,
            "a laboratory needs five turns to pay back"
        );
    }

    #[test]
    fn sells_idle_pawns_back_before_the_bank_closes() {
        let mut outlook = outlook();
        outlook.turn = outlook.max_turns - BANK_CLOSED_TURNS;
        outlook.stock = ItemCounts::new(0, 0, 1);
        let purchase = choose_purchase(&outlook, &holdings(1_000, 4, 1));
        assert_eq!((purchase.sold_peddlers, purchase.sold_caravans), (2, 0));
        let closed = Holdings {
            bank_open: false,
            ..holdings(1_000, 4, 1)
        };
        assert!(choose_purchase(&outlook, &closed).is_empty());
    }
}
