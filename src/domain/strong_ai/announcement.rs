//! Phase I.1: which fanatic, if any, the AI sends to the great hall.

use crate::domain::strong_ai::fanatic_odds::FanaticOdds;
use crate::domain::strong_ai::outlook::AiOutlook;
use crate::domain::{Cult, Fanatic, MagicItem};

/// Odds of dominating above which a fanatic is worth sending.
const SEND_ODDS: f64 = 0.6;

/// Worth given to a cult item nobody buys on the map yet, in écus.
const UNSOLD_ITEM_WORTH: i64 = 2_000;

/// Chooses the fanatic to send to the great hall.
///
/// For each cult, the AI weighs the odds that it dominates if it sends its strongest pawn of
/// that cult (the others are expected to send one pawn in two) by what an item of that cult
/// is worth on the map. It sends the best one when the odds are good, and always when it
/// holds as many fanatics as turns left, since they are worth nothing at the end.
///
/// # Returns
/// The index in `outlook.hand` of the pawn to send, or `None` to send nothing.
pub fn choose_announcement(outlook: &AiOutlook) -> Option<usize> {
    let values = outlook.item_values();
    let senders = outlook.rivals_with_fanatics.div_ceil(2);
    let (index, odds, _) = [Cult::Xephis, Cult::Yaboul]
        .into_iter()
        .filter_map(|cult| {
            let (index, fanatic) = strongest(&outlook.hand, cult)?;
            let odds = FanaticOdds::compute(&[fanatic], &outlook.unseen, senders).of(Some(cult));
            let worth = values
                .map_value(MagicItem::of_cult(cult))
                .unwrap_or(0)
                .max(UNSOLD_ITEM_WORTH);
            Some((index, odds, odds * worth as f64))
        })
        .max_by(|a, b| a.2.total_cmp(&b.2))?;
    let use_or_lose = outlook.hand.len() as i64 > outlook.turns_after();
    (odds >= SEND_ODDS || use_or_lose).then_some(index)
}

/// The strongest pawn of `cult` in `hand`, with its index.
fn strongest(hand: &[Fanatic], cult: Cult) -> Option<(usize, Fanatic)> {
    hand.iter()
        .copied()
        .enumerate()
        .filter(|(_, fanatic)| fanatic.cult == cult)
        .max_by_key(|(_, fanatic)| fanatic.strength)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::strong_ai::outlook::fixtures::outlook;

    #[test]
    fn sends_a_pawn_that_surely_dominates() {
        let mut outlook = outlook();
        outlook.hand = vec![Fanatic::new(Cult::Yaboul, 1), Fanatic::new(Cult::Xephis, 3)];
        outlook.unseen = vec![Fanatic::new(Cult::Xephis, 1)];
        outlook.rivals_with_fanatics = 1;
        assert_eq!(choose_announcement(&outlook), Some(1));
    }

    #[test]
    fn keeps_a_weak_pawn_early_and_spends_it_at_the_end() {
        let mut outlook = outlook();
        outlook.hand = vec![Fanatic::new(Cult::Yaboul, 1)];
        outlook.unseen = vec![Fanatic::new(Cult::Xephis, 3); 4];
        outlook.rivals_with_fanatics = 4;
        assert_eq!(choose_announcement(&outlook), None);
        outlook.turn = outlook.max_turns;
        assert_eq!(choose_announcement(&outlook), Some(0));
        outlook.hand.clear();
        assert_eq!(choose_announcement(&outlook), None);
    }
}
