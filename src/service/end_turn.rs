use sqlx::SqliteConnection;

use crate::db::{EventRecord, FanaticRecord, GameRecord, PlayerRecord};
use crate::domain::{Phase, TurnOrder};
use crate::error::AppError;

/// Closes the turn: discards the fanatics revealed in the great hall, resets the players'
/// turn state, then either starts the next turn with
/// the next first player or ends the game.
///
/// The records are updated in place; the caller saves `game`.
pub async fn end_turn(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    players: &mut [PlayerRecord],
) -> Result<(), AppError> {
    FanaticRecord::discard_hall(conn, game.id).await?;
    for player in players.iter_mut() {
        player.announcement_done = false;
        player.expense_done = false;
        player.moves_done = false;
        player.turn_expense = 0;
        player.save(conn).await?;
    }
    game.collective_expense = 0;
    game.city_sales_left = 0;
    game.city_passes = 0;
    game.active_seat = None;

    if game.turn >= game.max_turns {
        game.phase = Phase::Finished;
        return EventRecord::insert(conn, game.id, game.turn, &winners_message(players)).await;
    }
    game.turn += 1;
    game.first_seat = TurnOrder::new(players.len() as i64, game.first_seat)
        .next_turn()
        .first_seat();
    game.phase = Phase::Announcements;
    let first_name = players
        .iter()
        .find(|player| player.seat == game.first_seat)
        .map(|player| player.name.as_str())
        .unwrap_or_default();
    let message = format!("Tour {} : {first_name} joue en premier.", game.turn);
    EventRecord::insert(conn, game.id, game.turn, &message).await
}

/// Returns the French announcement of the richest player(s).
pub fn winners_message(players: &[PlayerRecord]) -> String {
    let best = players.iter().map(|player| player.money).max().unwrap_or(0);
    let winners: Vec<&str> = players
        .iter()
        .filter(|player| player.money == best)
        .map(|player| player.name.as_str())
        .collect();
    match winners.as_slice() {
        [winner] => format!("Fin de la partie : {winner} gagne avec {best} écus."),
        _ => format!(
            "Fin de la partie : {} gagnent ex aequo avec {best} écus.",
            winners.join(" et ")
        ),
    }
}
