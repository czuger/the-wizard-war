//! Phase I.2: what the AI pays into the collective expense.

use crate::domain::market::{CITY_QUOTA_EXPENSE_STEPS, CITY_QUOTA_TABLE, city_quota_row};
use crate::domain::rules::EXPENSE_STEP;
use crate::domain::strong_ai::outlook::AiOutlook;

/// Écus the AI keeps for its purchases.
const RESERVE: i64 = 1_000;

/// Players the AI expects to share the cost of opening the city.
const CONTRIBUTORS: i64 = 3;

/// The lowest city price, in écus: the least an item sold in the city brings.
const LOWEST_CITY_PRICE: i64 = 3_500;

/// Chooses the expense of the turn.
///
/// The expense only matters to sell in the city, so the AI pays nothing unless it holds or
/// is about to make more items than its pawns can take out before the end. It then pays its
/// share of the smallest expense that opens the city (Table 2), more or less as generous as
/// its temperament, twice that on the last turn,
/// never more than half what the extra items fetch in the city, and keeps a reserve.
///
/// # Returns
/// A multiple of [`EXPENSE_STEP`] between 0 and the AI's money.
pub fn choose_expense(outlook: &AiOutlook) -> i64 {
    let values = outlook.item_values();
    let surplus = (outlook.stock.total() + outlook.laboratories - values.trips()).max(0);
    if surplus == 0 {
        return 0;
    }
    let row = CITY_QUOTA_TABLE[city_quota_row(outlook.total_laboratories)];
    let Some(column) = row.iter().position(|&quota| quota > 0) else {
        return 0;
    };
    let opening = CITY_QUOTA_EXPENSE_STEPS[column];
    let per_player = opening / CONTRIBUTORS;
    let per_player = (per_player as f64 * outlook.temperament.expense_factor) as i64;
    let mut share = (per_player + EXPENSE_STEP - 1) / EXPENSE_STEP * EXPENSE_STEP;
    if outlook.turns_after() == 0 {
        share *= 2;
    }
    let worth = surplus * LOWEST_CITY_PRICE / 2;
    let affordable = (outlook.money - RESERVE).max(0);
    let amount = share.min(worth).min(affordable);
    amount / EXPENSE_STEP * EXPENSE_STEP
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ItemCounts;
    use crate::domain::strong_ai::outlook::fixtures::outlook;

    #[test]
    fn pays_nothing_when_the_pawns_carry_everything() {
        let outlook = outlook();
        assert_eq!(
            choose_expense(&outlook),
            0,
            "a peddler will come for the one item"
        );
    }

    #[test]
    fn pays_a_share_to_sell_its_surplus_in_the_city() {
        let mut outlook = outlook();
        outlook.stock = ItemCounts::new(2, 2, 2);
        let amount = choose_expense(&outlook);
        assert_eq!(amount, 700, "a third of 2,000 écus for 5 to 9 laboratories");
        outlook.turn = outlook.max_turns;
        outlook.sale_turns = 1;
        assert_eq!(choose_expense(&outlook), 1_400);
        outlook.money = 1_250;
        assert_eq!(choose_expense(&outlook), 200);
        assert_eq!(choose_expense(&outlook) % EXPENSE_STEP, 0);
    }
}
