use sqlx::{SqliteConnection, SqlitePool};

use crate::db::{EventRecord, GameRecord, PlayerRecord};
use crate::domain::Phase;
use crate::domain::rules::EXPENSE_STEP;
use crate::error::AppError;
use crate::service::player_action::PlayerAction;
use crate::service::write_transaction::begin_write;

/// Phase I.2: the player secretly pays into the collective expense envelope.
///
/// When every player has paid, production starts with the turn's first player.
///
/// # Arguments
/// * `amount` - écus paid, a multiple of 100, possibly 0.
///
/// # Errors
/// Fails when it is not the expenses phase, the player has already paid,
/// the amount is invalid or above their money, or on a database error.
pub async fn pay_expense(
    pool: &SqlitePool,
    game_id: i64,
    token: &str,
    amount: i64,
) -> Result<(), AppError> {
    let mut tx = begin_write(pool).await?;
    let PlayerAction {
        mut game,
        mut player,
    } = PlayerAction::load(&mut tx, game_id, token, Phase::Expenses).await?;
    record_expense(&mut tx, &mut game, &mut player, amount).await?;
    PlayerAction::finish(tx, game).await
}

/// Records an expense for a player of a game in the expenses phase.
///
/// The player is saved; the caller saves `game`.
pub(crate) async fn record_expense(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &mut PlayerRecord,
    amount: i64,
) -> Result<(), AppError> {
    if player.expense_done {
        return Err(AppError::AlreadyDone);
    }
    if amount < 0 || amount % EXPENSE_STEP != 0 {
        return Err(AppError::InvalidExpense);
    }
    if amount > player.money {
        return Err(AppError::InsufficientFunds);
    }

    player.money -= amount;
    player.turn_expense = amount;
    player.expense_done = true;
    player.save(conn).await?;
    game.collective_expense += amount;
    let message = format!("{} glisse sa contribution dans l'enveloppe.", player.name);
    EventRecord::insert(conn, game.id, game.turn, &message).await?;

    let players = PlayerRecord::list_for_game(conn, game.id).await?;
    if players.iter().all(|other| other.expense_done) {
        game.phase = Phase::Production;
        game.active_seat = Some(game.first_seat);
    }
    Ok(())
}
