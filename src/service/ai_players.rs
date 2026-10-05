use sqlx::SqliteConnection;

use crate::db::{FanaticRecord, GameRecord, PlayerRecord, RebelRecord};
use crate::domain::ai_strategy::{
    choose_announcement, choose_conversion, choose_expense, choose_production, preferred_cult,
};
use crate::domain::rebel_movement::random_destination;
use crate::domain::{Fanatic, Phase};
use crate::error::AppError;
use crate::service::announce_fanatic::record_announcement;
use crate::service::board_loader::load_board_state;
use crate::service::convert_site::record_conversion;
use crate::service::move_rebels::{RebelMove, record_rebel_moves};
use crate::service::pay_expense::record_expense;
use crate::service::produce_items::{record_production, required_items};

/// Lets the computer-controlled alchemists act until the human player has to play or the
/// game is over.
///
/// The players are saved; the caller saves `game`.
///
/// # Errors
/// Fails on a database error, or if an AI decision is refused, which would be a bug.
pub async fn play_ai_players(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
) -> Result<(), AppError> {
    while let Some(mut player) = next_ai_player(conn, game).await? {
        match game.phase {
            Phase::Announcements => announce(conn, game, &mut player).await?,
            Phase::Expenses => {
                let amount = choose_expense(player.money, &mut rand::rng());
                record_expense(conn, game, &mut player, amount).await?;
            }
            Phase::Conversions => convert(conn, game, &player).await?,
            Phase::RebelMovement => move_rebels(conn, game, &player).await?,
            Phase::Production => produce(conn, game, &mut player).await?,
            Phase::Finished => break,
        }
    }
    Ok(())
}

/// Returns a computer-controlled alchemist expected to act in the current phase, if any.
async fn next_ai_player(
    conn: &mut SqliteConnection,
    game: &GameRecord,
) -> Result<Option<PlayerRecord>, AppError> {
    let players = PlayerRecord::list_for_game(conn, game.id).await?;
    Ok(players.into_iter().find(|player| {
        player.is_ai
            && match game.phase {
                Phase::Announcements => !player.announcement_done,
                Phase::Expenses => !player.expense_done,
                Phase::Conversions | Phase::RebelMovement | Phase::Production => {
                    game.active_seat == Some(player.seat)
                }
                Phase::Finished => false,
            }
    }))
}

async fn announce(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &mut PlayerRecord,
) -> Result<(), AppError> {
    let available: Vec<FanaticRecord> = FanaticRecord::list_for_player(conn, player.id)
        .await?
        .into_iter()
        .filter(|record| !record.in_hall)
        .collect();
    let hand: Vec<Fanatic> = available.iter().map(|record| record.fanatic).collect();
    let fanatic_id = choose_announcement(&hand, &mut rand::rng())
        .and_then(|index| available.get(index))
        .map(|record| record.id);
    record_announcement(conn, game, player, fanatic_id).await
}

/// Produces items backing the cult of the pawn the AI sent to the hall this turn, if any.
async fn produce(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &mut PlayerRecord,
) -> Result<(), AppError> {
    let backed_cult = FanaticRecord::list_for_player(conn, player.id)
        .await?
        .into_iter()
        .find(|record| record.in_hall)
        .map(|record| record.fanatic.cult);
    let count = required_items(player.laboratories, &game.supply);
    let items = choose_production(count, &game.supply, backed_cult);
    record_production(conn, game, player, items).await
}

/// Spreads the cult the AI's fanatics are strongest in.
async fn convert(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &PlayerRecord,
) -> Result<(), AppError> {
    let hand: Vec<Fanatic> = FanaticRecord::list_for_player(conn, player.id)
        .await?
        .into_iter()
        .map(|record| record.fanatic)
        .collect();
    let board = load_board_state(conn, game.id).await?;
    let conversion = choose_conversion(&board, game.turn, preferred_cult(&hand), &mut rand::rng());
    record_conversion(conn, game, player, conversion).await
}

/// Sends every rebels pawn to a random adjacent village.
async fn move_rebels(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &PlayerRecord,
) -> Result<(), AppError> {
    let rebels = RebelRecord::list_for_game(conn, game.id).await?;
    let moves = random_moves(&rebels);
    record_rebel_moves(conn, game, player, &moves).await
}

/// Kept synchronous: the thread-local RNG must not live across an `.await`.
fn random_moves(rebels: &[RebelRecord]) -> Vec<RebelMove> {
    let mut rng = rand::rng();
    rebels
        .iter()
        .filter_map(|rebel| {
            random_destination(rebel.site, &mut rng).map(|to| RebelMove {
                rebel_id: rebel.id,
                to,
            })
        })
        .collect()
}
