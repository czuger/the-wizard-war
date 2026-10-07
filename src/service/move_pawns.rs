use std::collections::BTreeSet;

use sqlx::{SqliteConnection, SqlitePool};

use crate::db::{EventRecord, GameRecord, PawnRecord, PlayerRecord, RebelRecord, SaleRecord};
use crate::domain::board::site_by_id;
use crate::domain::travel::destinations;
use crate::domain::{ItemCounts, Location, LogSection, PawnOrder, Phase};
use crate::error::AppError;
use crate::service::board_loader::load_board_state;
use crate::service::pawn_fight::fight;
use crate::service::pawn_rules::sale_offer;
use crate::service::peddler_turns::finish_peddler_turn;
use crate::service::player_action::PlayerAction;
use crate::service::write_transaction::begin_write;

/// Phase V.1: the active player moves their peddlers and caravans.
///
/// A pawn ending its move on rebels fights them. The player then sells, or the next
/// player moves when none of their pawns can sell.
///
/// # Arguments
/// * `orders` - one order per pawn that moves; the other pawns stay where they are.
///
/// # Errors
/// Fails when it is not phase V or not the player's turn, when the player has already
/// moved, when an order breaks a rule, or on a database error.
pub async fn move_pawns(
    pool: &SqlitePool,
    game_id: i64,
    token: &str,
    orders: Vec<PawnOrder>,
) -> Result<(), AppError> {
    let mut tx = begin_write(pool).await?;
    let PlayerAction {
        mut game,
        mut player,
    } = PlayerAction::load(&mut tx, game_id, token, Phase::Peddlers).await?;
    record_pawn_moves(&mut tx, &mut game, &mut player, &orders).await?;
    PlayerAction::finish(tx, game).await
}

/// Records the moves of the active player of a game in phase V and resolves the fights.
///
/// The player is saved; the caller saves `game`.
pub(crate) async fn record_pawn_moves(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &mut PlayerRecord,
    orders: &[PawnOrder],
) -> Result<(), AppError> {
    if game.active_seat != Some(player.seat) {
        return Err(AppError::NotYourTurn);
    }
    if player.moves_done {
        return Err(AppError::AlreadyDone);
    }
    let pawns: Vec<PawnRecord> = PawnRecord::list_for_game(conn, game.id)
        .await?
        .into_iter()
        .filter(|pawn| pawn.player_id == player.id)
        .collect();
    check_orders(&pawns, orders, game.turn, player.stock)?;
    let mut moved = Vec::new();
    for order in orders {
        if let Some(pawn) = pawns.iter().find(|pawn| pawn.id == order.pawn_id) {
            moved.push(apply_order(conn, game, player, *pawn, order).await?);
        }
    }
    player.moves_done = true;
    player.save(conn).await?;
    for pawn in moved {
        fight_rebels_on_arrival(conn, game, &player.name, pawn).await?;
    }
    if !can_sell(conn, game, player.id).await? {
        finish_peddler_turn(conn, game, player.seat).await?;
    }
    Ok(())
}

/// Checks every order against the rules of phase V.1.
fn check_orders(
    pawns: &[PawnRecord],
    orders: &[PawnOrder],
    turn: i64,
    stock: ItemCounts,
) -> Result<(), AppError> {
    let mut stock = stock;
    let mut seen = BTreeSet::new();
    for order in orders {
        let pawn = pawns
            .iter()
            .find(|pawn| pawn.id == order.pawn_id)
            .ok_or(AppError::InvalidPawnMove)?;
        let reachable = destinations(pawn.kind, pawn.location).contains(&order.to);
        if !seen.insert(pawn.id) || !pawn.can_move(turn) || !reachable {
            return Err(AppError::InvalidPawnMove);
        }
        match (pawn.location, order.item) {
            (Location::City, None) => return Err(AppError::CargoRequired),
            (Location::City, Some(item)) if stock.get(item) <= 0 => {
                return Err(AppError::NotInStock);
            }
            (Location::City, Some(item)) => stock = stock.with(item, stock.get(item) - 1),
            (Location::Site(_), Some(_)) => return Err(AppError::InvalidPawnMove),
            (Location::Site(_), None) => {}
        }
    }
    Ok(())
}

/// Moves one pawn, loading its item when it leaves the city and unloading it when it comes
/// back. The caller saves `player`.
///
/// # Returns
/// The pawn after its move.
async fn apply_order(
    conn: &mut SqliteConnection,
    game: &GameRecord,
    player: &mut PlayerRecord,
    mut pawn: PawnRecord,
    order: &PawnOrder,
) -> Result<PawnRecord, AppError> {
    if let Some(item) = order.item {
        player.stock = player.stock.with(item, player.stock.get(item) - 1);
        pawn.item = Some(item);
    }
    pawn.location = order.to;
    let message = match order.to {
        Location::City => {
            if let Some(item) = pawn.item.take() {
                player.stock = player.stock.with(item, player.stock.get(item) + 1);
            }
            format!(
                "{} : {} rentre en ville.",
                player.name,
                pawn.kind.with_article()
            )
        }
        Location::Site(site) => format!(
            "{} : {} va à {}.",
            player.name,
            pawn.kind.with_article(),
            site_by_id(site).map_or("?", |site| site.name)
        ),
    };
    pawn.save(conn).await?;
    EventRecord::insert(conn, game.id, game.turn, LogSection::Peddlers, &message).await?;
    Ok(pawn)
}

/// A pawn ending its move on a village with rebels fights one of them; its owner rolls.
async fn fight_rebels_on_arrival(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    owner: &str,
    pawn: PawnRecord,
) -> Result<(), AppError> {
    let Location::Site(site) = pawn.location else {
        return Ok(());
    };
    let rebels = RebelRecord::list_for_game(conn, game.id).await?;
    match rebels.iter().find(|rebel| rebel.site == site) {
        Some(rebel) => fight(conn, game, pawn, owner, rebel, game.turn + 1).await,
        None => Ok(()),
    }
}

/// Whether one of the player's pawns can sell where it stands.
pub(crate) async fn can_sell(
    conn: &mut SqliteConnection,
    game: &GameRecord,
    player_id: i64,
) -> Result<bool, AppError> {
    let board = load_board_state(conn, game.id).await?;
    let sold = SaleRecord::sites_sold(conn, game.id, game.turn).await?;
    let pawns = PawnRecord::list_for_game(conn, game.id).await?;
    Ok(pawns
        .iter()
        .filter(|pawn| pawn.player_id == player_id)
        .any(|pawn| sale_offer(pawn, &board, &sold).is_some()))
}
