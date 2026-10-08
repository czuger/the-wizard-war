use sqlx::{SqliteConnection, SqlitePool};

use crate::db::{EventRecord, GameRecord, PawnRecord, PlayerRecord};
use crate::domain::board::site_by_id;
use crate::domain::{LogSection, Phase};
use crate::error::AppError;
use crate::service::pawn_rules::can_transfer;
use crate::service::player_action::PlayerAction;
use crate::service::write_transaction::begin_write;

/// Phase V.2: once their pawns have moved, the active player passes the item of one of their
/// pawns to another of their pawns on the same site, at once; if the receiving pawn carries
/// an item too, the two pawns swap. The player then goes on selling.
///
/// # Arguments
/// * `from_pawn_id` - the pawn that gives its item.
/// * `to_pawn_id` - the pawn that receives it.
///
/// # Errors
/// Fails when it is not phase V, not the player's turn or the player has not moved yet,
/// when the pawns may not pass an item between them, or on a database error.
pub async fn transfer_item(
    pool: &SqlitePool,
    game_id: i64,
    token: &str,
    from_pawn_id: i64,
    to_pawn_id: i64,
) -> Result<(), AppError> {
    let mut tx = begin_write(pool).await?;
    let PlayerAction { mut game, player } =
        PlayerAction::load(&mut tx, game_id, token, Phase::Peddlers).await?;
    record_transfer(&mut tx, &mut game, &player, from_pawn_id, to_pawn_id).await?;
    PlayerAction::finish(tx, game).await
}

/// Moves the item between the two pawns and logs it, without telling the other players
/// which items they carry.
async fn record_transfer(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &PlayerRecord,
    from_pawn_id: i64,
    to_pawn_id: i64,
) -> Result<(), AppError> {
    if game.active_seat != Some(player.seat) {
        return Err(AppError::NotYourTurn);
    }
    if !player.moves_done {
        return Err(AppError::WrongPhase);
    }
    let pawns = PawnRecord::list_for_game(conn, game.id).await?;
    let mine = |id: i64| {
        pawns
            .iter()
            .find(|pawn| pawn.id == id && pawn.player_id == player.id)
            .copied()
            .ok_or(AppError::InvalidTransfer)
    };
    let (mut from, mut to) = (mine(from_pawn_id)?, mine(to_pawn_id)?);
    if !can_transfer(&from, &to) {
        return Err(AppError::InvalidTransfer);
    }
    let swap = to.item.is_some();
    std::mem::swap(&mut from.item, &mut to.item);
    from.save(conn).await?;
    to.save(conn).await?;
    let site = from
        .location
        .site()
        .and_then(site_by_id)
        .map_or("?", |site| site.name);
    let message = format!("{} : {} à {site}.", player.name, describe(&from, &to, swap));
    EventRecord::insert(conn, game.id, game.turn, LogSection::Peddlers, &message).await
}

/// French account of the transfer, e.g. « le colporteur passe son objet à la caravane ».
fn describe(from: &PawnRecord, to: &PawnRecord, swap: bool) -> String {
    let same_kind = from.kind == to.kind;
    match (swap, same_kind) {
        (true, true) => format!("deux {}s échangent leurs objets", from.kind.label()),
        (true, false) => format!(
            "{} et {} échangent leurs objets",
            from.kind.with_article(),
            to.kind.with_article()
        ),
        (false, true) => format!(
            "{} passe son objet à {}",
            from.kind.with_article(),
            from.kind.another()
        ),
        (false, false) => format!(
            "{} passe son objet {}",
            from.kind.with_article(),
            to.kind.to_article()
        ),
    }
}
