use sqlx::SqliteConnection;

use crate::db::{GameRecord, PawnRecord, PlayerRecord};
use crate::domain::Phase;
use crate::error::AppError;
use crate::service::city_sales::start_city_sales;
use crate::service::pawn_rules::is_movable;
use crate::service::seat_cycle::next_eligible_seat;

/// Starts phase V with the first player of the turn who has a pawn to move, or goes on to
/// the city sales when nobody has.
///
/// The players are saved; the caller saves `game`.
pub(crate) async fn start_peddlers(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
) -> Result<(), AppError> {
    for mut player in PlayerRecord::list_for_game(conn, game.id).await? {
        player.moves_done = false;
        player.save(conn).await?;
    }
    game.phase = Phase::Peddlers;
    advance(conn, game, None).await
}

/// Hands phase V to the next player who has a pawn to move, or starts the city sales.
///
/// The caller saves `game`.
pub(crate) async fn finish_peddler_turn(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    seat: i64,
) -> Result<(), AppError> {
    advance(conn, game, Some(seat)).await
}

async fn advance(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    after: Option<i64>,
) -> Result<(), AppError> {
    let players = PlayerRecord::list_for_game(conn, game.id).await?;
    let pawns = PawnRecord::list_for_game(conn, game.id).await?;
    let has_work = |seat: i64| {
        players
            .iter()
            .find(|player| player.seat == seat)
            .is_some_and(|player| {
                pawns.iter().any(|pawn| {
                    pawn.player_id == player.id && is_movable(pawn, game.turn, &player.stock)
                })
            })
    };
    match next_eligible_seat(game, after, has_work) {
        Some(seat) => {
            game.active_seat = Some(seat);
            Ok(())
        }
        None => start_city_sales(conn, game).await,
    }
}
