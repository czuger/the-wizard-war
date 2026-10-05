use sqlx::{SqliteConnection, SqlitePool};

use crate::db::{EventRecord, GameRecord, PlayerRecord};
use crate::domain::market::{city_price, city_sale_quota};
use crate::domain::rules::PLAYER_COUNT;
use crate::domain::{MagicItem, Phase};
use crate::error::AppError;
use crate::service::player_action::PlayerAction;
use crate::service::purchases::start_purchases;
use crate::service::seat_cycle::next_eligible_seat;
use crate::service::write_transaction::begin_write;

/// Starts phase VI: reveals the collective expense, which goes to the bank, and works out
/// from Table 2 how many items the city buys. Purchases follow when nobody can sell.
///
/// The caller saves `game`.
pub(crate) async fn start_city_sales(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
) -> Result<(), AppError> {
    let players = PlayerRecord::list_for_game(conn, game.id).await?;
    let laboratories: i64 = players.iter().map(|player| player.laboratories).sum();
    let quota = city_sale_quota(laboratories, game.collective_expense);
    let message = format!(
        "Dépense collective du tour : {} écus. Avec {laboratories} laboratoires, la ville achète jusqu'à {quota} objet(s).",
        game.collective_expense
    );
    EventRecord::insert(conn, game.id, game.turn, &message).await?;
    game.collective_expense = 0;
    game.phase = Phase::CitySales;
    game.city_sales_left = quota;
    game.city_passes = 0;
    let has_stock = |seat: i64| {
        players
            .iter()
            .any(|player| player.seat == seat && player.stock.total() > 0)
    };
    match next_eligible_seat(game, None, has_stock) {
        Some(seat) if quota > 0 => {
            game.active_seat = Some(seat);
            Ok(())
        }
        _ => start_purchases(conn, game).await,
    }
}

/// Phase VI: the active player sells one item from their stock at the city price, or passes.
///
/// Players are asked in turn, round after round, until the city has bought its quota or
/// every player with a stock has passed in a row.
///
/// # Arguments
/// * `item` - the item sold, or `None` to pass.
///
/// # Errors
/// Fails when it is not phase VI or not the player's turn, when the item is not in their
/// stock, or on a database error.
pub async fn sell_in_city(
    pool: &SqlitePool,
    game_id: i64,
    token: &str,
    item: Option<MagicItem>,
) -> Result<(), AppError> {
    let mut tx = begin_write(pool).await?;
    let PlayerAction {
        mut game,
        mut player,
    } = PlayerAction::load(&mut tx, game_id, token, Phase::CitySales).await?;
    record_city_sale(&mut tx, &mut game, &mut player, item).await?;
    PlayerAction::finish(tx, game).await
}

/// Records the city sale or pass of the active player of a game in phase VI.
///
/// The player is saved; the caller saves `game`.
pub(crate) async fn record_city_sale(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &mut PlayerRecord,
    item: Option<MagicItem>,
) -> Result<(), AppError> {
    if game.active_seat != Some(player.seat) {
        return Err(AppError::NotYourTurn);
    }
    let message = match item {
        Some(item) if player.stock.get(item) <= 0 => return Err(AppError::NotInStock),
        Some(item) => {
            let price = city_price(item);
            player.stock = player.stock.with(item, player.stock.get(item) - 1);
            player.money += price;
            player.save(conn).await?;
            game.supply = game.supply.with(item, game.supply.get(item) + 1);
            game.city_sales_left -= 1;
            game.city_passes = 0;
            format!(
                "{} vend {} en ville pour {price} écus.",
                player.name,
                item.with_article()
            )
        }
        None => {
            game.city_passes += 1;
            format!("{} ne vend rien en ville.", player.name)
        }
    };
    EventRecord::insert(conn, game.id, game.turn, &message).await?;
    advance(conn, game, player.seat).await
}

/// Asks the next player with a stock, going round the table, or starts the purchases.
async fn advance(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    seat: i64,
) -> Result<(), AppError> {
    let players = PlayerRecord::list_for_game(conn, game.id).await?;
    let sellers: Vec<i64> = players
        .iter()
        .filter(|player| player.stock.total() > 0)
        .map(|player| player.seat)
        .collect();
    if game.city_sales_left <= 0 || game.city_passes >= sellers.len() as i64 {
        return start_purchases(conn, game).await;
    }
    let count = PLAYER_COUNT as i64;
    game.active_seat = (1..=count)
        .map(|step| (seat + step) % count)
        .find(|next| sellers.contains(next));
    Ok(())
}
