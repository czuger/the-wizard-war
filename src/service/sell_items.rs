use std::collections::BTreeSet;

use sqlx::{SqliteConnection, SqlitePool};

use crate::db::{EventRecord, GameRecord, PawnRecord, PlayerRecord, SaleRecord};
use crate::domain::board::site_by_id;
use crate::domain::market::buys_without_limit;
use crate::domain::{Phase, SiteId};
use crate::error::AppError;
use crate::service::board_loader::load_board_state;
use crate::service::pawn_rules::sale_offer;
use crate::service::peddler_turns::finish_peddler_turn;
use crate::service::player_action::PlayerAction;
use crate::service::write_transaction::begin_write;

/// A sale checked against the rules, ready to be recorded.
struct Sale {
    pawn: PawnRecord,
    site: SiteId,
    price: i64,
}

/// Phase V.2: once their pawns have moved, the active player sells the items of the pawns
/// they choose where those pawns stand. The next player then moves.
///
/// # Arguments
/// * `pawn_ids` - the pawns that sell; possibly none.
///
/// # Errors
/// Fails when it is not phase V, not the player's turn or the player has not moved yet,
/// when a pawn cannot sell where it stands, or on a database error.
pub async fn sell_items(
    pool: &SqlitePool,
    game_id: i64,
    token: &str,
    pawn_ids: Vec<i64>,
) -> Result<(), AppError> {
    let mut tx = begin_write(pool).await?;
    let PlayerAction {
        mut game,
        mut player,
    } = PlayerAction::load(&mut tx, game_id, token, Phase::Peddlers).await?;
    record_sales(&mut tx, &mut game, &mut player, &pawn_ids).await?;
    PlayerAction::finish(tx, game).await
}

/// Records the sales of the active player of a game in phase V, then hands over.
///
/// The player is saved; the caller saves `game`.
pub(crate) async fn record_sales(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &mut PlayerRecord,
    pawn_ids: &[i64],
) -> Result<(), AppError> {
    if game.active_seat != Some(player.seat) {
        return Err(AppError::NotYourTurn);
    }
    if !player.moves_done {
        return Err(AppError::WrongPhase);
    }
    let sales = check_sales(conn, game, player.id, pawn_ids).await?;
    for mut sale in sales {
        let Some(item) = sale.pawn.item.take() else {
            continue;
        };
        player.money += sale.price;
        game.supply = game.supply.with(item, game.supply.get(item) + 1);
        sale.pawn.save(conn).await?;
        SaleRecord::insert(conn, game.id, game.turn, sale.site).await?;
        let message = format!(
            "{} vend {} à {} pour {} écus.",
            player.name,
            item.with_article(),
            site_by_id(sale.site).map_or("?", |site| site.name),
            sale.price
        );
        EventRecord::insert(conn, game.id, game.turn, &message).await?;
    }
    player.save(conn).await?;
    finish_peddler_turn(conn, game, player.seat).await
}

/// Checks that each pawn belongs to the player and may sell where it stands, one item per
/// site except in monasteries.
async fn check_sales(
    conn: &mut SqliteConnection,
    game: &GameRecord,
    player_id: i64,
    pawn_ids: &[i64],
) -> Result<Vec<Sale>, AppError> {
    let board = load_board_state(conn, game.id).await?;
    let mut sold = SaleRecord::sites_sold(conn, game.id, game.turn).await?;
    let pawns = PawnRecord::list_for_game(conn, game.id).await?;
    let mut seen = BTreeSet::new();
    let mut sales = Vec::new();
    for &id in pawn_ids {
        let pawn = pawns
            .iter()
            .find(|pawn| pawn.id == id && pawn.player_id == player_id)
            .ok_or(AppError::InvalidSale)?;
        let price = sale_offer(pawn, &board, &sold).ok_or(AppError::InvalidSale)?;
        let site = pawn.location.site().ok_or(AppError::InvalidSale)?;
        if !seen.insert(id) {
            return Err(AppError::InvalidSale);
        }
        if site_by_id(site).is_some_and(|site| !buys_without_limit(site)) {
            sold.push(site);
        }
        sales.push(Sale {
            pawn: *pawn,
            site,
            price,
        });
    }
    Ok(sales)
}
