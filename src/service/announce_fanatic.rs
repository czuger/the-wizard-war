use sqlx::SqlitePool;

use crate::db::{EventRecord, FanaticRecord, PlayerRecord};
use crate::domain::Phase;
use crate::error::AppError;
use crate::service::player_action::PlayerAction;
use crate::service::write_transaction::begin_write;

/// Phase I.1: the player sends one of their fanatics to the great hall, or none.
///
/// When every player has announced, the game moves on to the expenses.
///
/// # Arguments
/// * `fanatic_id` - the pawn to send, or `None` to send nothing this turn.
///
/// # Errors
/// Fails when it is not the announcements phase, the player has already announced,
/// the pawn is not theirs or already in the hall, or on a database error.
pub async fn announce_fanatic(
    pool: &SqlitePool,
    game_id: i64,
    token: &str,
    fanatic_id: Option<i64>,
) -> Result<(), AppError> {
    let mut tx = begin_write(pool).await?;
    let PlayerAction {
        mut game,
        mut player,
    } = PlayerAction::load(&mut tx, game_id, token, Phase::Announcements).await?;
    if player.announcement_done {
        return Err(AppError::AlreadyDone);
    }

    let message = match fanatic_id {
        Some(id) => {
            let owned = FanaticRecord::list_for_player(&mut tx, player.id).await?;
            if !owned
                .iter()
                .any(|record| record.id == id && !record.in_hall)
            {
                return Err(AppError::FanaticNotAvailable);
            }
            FanaticRecord::move_to_hall(&mut tx, id).await?;
            format!("{} envoie un fanatique dans la grande salle.", player.name)
        }
        None => format!("{} n'envoie aucun fanatique.", player.name),
    };
    player.announcement_done = true;
    player.save(&mut tx).await?;
    EventRecord::insert(&mut tx, game.id, game.turn, &message).await?;

    let players = PlayerRecord::list_for_game(&mut tx, game.id).await?;
    if players.iter().all(|other| other.announcement_done) {
        game.phase = Phase::Expenses;
    }
    game.save(&mut tx).await?;
    tx.commit().await?;
    Ok(())
}
