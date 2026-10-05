use sqlx::{SqliteConnection, SqlitePool};

use crate::db::{GameRecord, PawnRecord, PlayerRecord, RebelRecord};
use crate::domain::{Location, Phase};
use crate::error::AppError;
use crate::service::pawn_fight::fight;
use crate::service::player_action::PlayerAction;
use crate::service::write_transaction::begin_write;

/// A rebels pawn that has reached peddlers or caravans, and the pawns it may attack.
#[derive(Debug, Clone)]
pub(crate) struct PendingFight {
    /// The attacking rebels pawn.
    pub rebel: RebelRecord,
    /// The peddlers and caravans on its village.
    pub targets: Vec<PawnRecord>,
}

/// Phase II.2: the turn's first player chooses the peddler or caravan the next rebels pawn
/// attacks; the die decides the fight.
///
/// # Errors
/// Fails when no fight is pending, the player is not the turn's first player, the pawn is
/// not on the rebels' village, or on a database error.
pub async fn attack_pawn(
    pool: &SqlitePool,
    game_id: i64,
    token: &str,
    pawn_id: i64,
) -> Result<(), AppError> {
    let mut tx = begin_write(pool).await?;
    let PlayerAction { mut game, player } =
        PlayerAction::load(&mut tx, game_id, token, Phase::RebelFights).await?;
    record_rebel_attack(&mut tx, &mut game, &player, pawn_id).await?;
    PlayerAction::finish(tx, game).await
}

/// Resolves the next pending fight with the target chosen by the turn's first player.
///
/// The caller saves `game`.
pub(crate) async fn record_rebel_attack(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &PlayerRecord,
    pawn_id: i64,
) -> Result<(), AppError> {
    if game.active_seat != Some(player.seat) {
        return Err(AppError::NotYourTurn);
    }
    let pending = pending_fight(conn, game.id)
        .await?
        .ok_or(AppError::WrongPhase)?;
    let target = pending
        .targets
        .into_iter()
        .find(|pawn| pawn.id == pawn_id)
        .ok_or(AppError::InvalidFightTarget)?;
    let players = PlayerRecord::list_for_game(conn, game.id).await?;
    let owner = players
        .iter()
        .find(|owner| owner.id == target.player_id)
        .map_or("?", |owner| owner.name.as_str());
    fight(conn, game, target, owner, &pending.rebel, game.turn).await?;
    continue_rebel_fights(conn, game).await
}

/// Goes on with the next fight, or starts production once every fight is over.
///
/// The caller saves `game`.
pub(crate) async fn continue_rebel_fights(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
) -> Result<(), AppError> {
    game.active_seat = Some(game.first_seat);
    if pending_fight(conn, game.id).await?.is_some() {
        game.phase = Phase::RebelFights;
        return Ok(());
    }
    for rebel in RebelRecord::list_for_game(conn, game.id).await? {
        if rebel.must_fight {
            RebelRecord::set_must_fight(conn, rebel.id, false).await?;
        }
    }
    game.phase = Phase::Production;
    Ok(())
}

/// Returns the next rebels pawn that still has to fight, oldest first. Rebels whose village
/// has no peddler or caravan left do not fight.
pub(crate) async fn pending_fight(
    conn: &mut SqliteConnection,
    game_id: i64,
) -> Result<Option<PendingFight>, AppError> {
    let rebels = RebelRecord::list_for_game(conn, game_id).await?;
    let pawns = PawnRecord::list_for_game(conn, game_id).await?;
    Ok(rebels
        .into_iter()
        .filter(|rebel| rebel.must_fight)
        .find_map(|rebel| {
            let targets: Vec<PawnRecord> = pawns
                .iter()
                .filter(|pawn| pawn.location == Location::Site(rebel.site))
                .copied()
                .collect();
            (!targets.is_empty()).then_some(PendingFight { rebel, targets })
        }))
}
