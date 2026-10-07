use sqlx::{SqliteConnection, SqlitePool};

use crate::db::{EventRecord, GameRecord, PlayerRecord};
use crate::domain::{ItemCounts, LogSection, Phase, TurnOrder};
use crate::error::AppError;
use crate::service::peddler_turns::start_peddlers;
use crate::service::player_action::PlayerAction;
use crate::service::resolve_fanatics::resolve_fanatics;
use crate::service::write_transaction::begin_write;

/// Phase III: the active player secretly takes items from the supply, one per laboratory.
///
/// After the last player of the turn, the fanatics act (phase IV) and the peddlers and
/// caravans move (phase V).
///
/// # Arguments
/// * `items` - how many items of each kind the player takes.
///
/// # Errors
/// Fails when it is not the production phase or not the player's turn, when the supply
/// does not hold the items, when the player does not take exactly as many items as they
/// have laboratories (or what is left in the supply), or on a database error.
pub async fn produce_items(
    pool: &SqlitePool,
    game_id: i64,
    token: &str,
    items: ItemCounts,
) -> Result<(), AppError> {
    let mut tx = begin_write(pool).await?;
    let PlayerAction {
        mut game,
        mut player,
    } = PlayerAction::load(&mut tx, game_id, token, Phase::Production).await?;
    record_production(&mut tx, &mut game, &mut player, items).await?;
    PlayerAction::finish(tx, game).await
}

/// Records the production of the active player of a game in the production phase; after
/// the last one, resolves the fanatics and starts phase V.
///
/// The players are saved; the caller saves `game`.
pub(crate) async fn record_production(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &mut PlayerRecord,
    items: ItemCounts,
) -> Result<(), AppError> {
    if game.active_seat != Some(player.seat) {
        return Err(AppError::NotYourTurn);
    }
    if !items.is_non_negative() || !items.fits_within(&game.supply) {
        return Err(AppError::SupplyExhausted);
    }
    let expected = required_items(player.laboratories, &game.supply);
    if items.total() != expected {
        return Err(AppError::WrongProductionCount { expected });
    }

    game.supply = game.supply.minus(items);
    player.production = items;
    player.save(conn).await?;
    let message = format!(
        "{} fabrique {} objet(s) magique(s).",
        player.name,
        items.total()
    );
    EventRecord::insert(conn, game.id, game.turn, LogSection::Production, &message).await?;

    let mut players = PlayerRecord::list_for_game(conn, game.id).await?;
    let order = TurnOrder::new(players.len() as i64, game.first_seat);
    game.active_seat = order.next_after(player.seat);
    if game.active_seat.is_none() {
        resolve_fanatics(conn, game, &mut players).await?;
        start_peddlers(conn, game).await?;
    }
    Ok(())
}

/// Returns how many items a player must take: one per laboratory, as long as the supply lasts.
pub fn required_items(laboratories: i64, supply: &ItemCounts) -> i64 {
    laboratories.min(supply.total())
}
