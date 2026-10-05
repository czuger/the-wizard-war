use crate::db::GameRecord;
use crate::domain::TurnOrder;
use crate::domain::rules::PLAYER_COUNT;

/// The seats of the current turn, from its first player, going left.
pub(crate) fn seats_in_turn_order(game: &GameRecord) -> Vec<i64> {
    let order = TurnOrder::new(PLAYER_COUNT as i64, game.first_seat);
    std::iter::successors(Some(order.first_seat()), |seat| order.next_after(*seat)).collect()
}

/// The next seat of the turn that has something to do in a sequential phase.
///
/// # Arguments
/// * `after` - the seat that just played, or `None` to start from the turn's first player.
/// * `eligible` - whether a seat has something to do.
///
/// # Returns
/// `None` once every later seat of the turn has been considered.
pub(crate) fn next_eligible_seat(
    game: &GameRecord,
    after: Option<i64>,
    eligible: impl Fn(i64) -> bool,
) -> Option<i64> {
    let seats = seats_in_turn_order(game);
    let start = match after {
        None => 0,
        Some(seat) => seats.iter().position(|&s| s == seat)? + 1,
    };
    seats.into_iter().skip(start).find(|&seat| eligible(seat))
}
