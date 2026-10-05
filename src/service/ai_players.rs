use sqlx::SqliteConnection;

use crate::db::{FanaticRecord, GameRecord, PawnRecord, PlayerRecord, RebelRecord, SaleRecord};
use crate::domain::ai_strategy::{
    choose_announcement, choose_conversion, choose_expense, choose_production, preferred_cult,
};
use crate::domain::ai_trading::{
    MovablePawn, choose_city_sale, choose_fight_target, choose_pawn_moves, choose_purchase,
};
use crate::domain::board::site_by_id;
use crate::domain::market::buys_without_limit;
use crate::domain::rebel_movement::random_destination;
use crate::domain::{Fanatic, Phase};
use crate::error::AppError;
use crate::service::announce_fanatic::record_announcement;
use crate::service::board_loader::load_board_state;
use crate::service::city_sales::record_city_sale;
use crate::service::convert_site::record_conversion;
use crate::service::move_pawns::record_pawn_moves;
use crate::service::move_rebels::{RebelMove, record_rebel_moves};
use crate::service::pawn_rules::{is_movable, sale_offer};
use crate::service::pay_expense::record_expense;
use crate::service::produce_items::{record_production, required_items};
use crate::service::purchases::{holdings_of, record_purchase};
use crate::service::rebel_fights::{pending_fight, record_rebel_attack};
use crate::service::sell_items::record_sales;

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
            Phase::RebelFights => attack(conn, game, &player).await?,
            Phase::Production => produce(conn, game, &mut player).await?,
            Phase::Peddlers if !player.moves_done => move_pawns(conn, game, &mut player).await?,
            Phase::Peddlers => sell(conn, game, &mut player).await?,
            Phase::CitySales => {
                let item = choose_city_sale(player.stock);
                record_city_sale(conn, game, &mut player, item).await?;
            }
            Phase::Purchases => buy(conn, game, &mut player).await?,
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
                Phase::Conversions
                | Phase::RebelMovement
                | Phase::RebelFights
                | Phase::Production
                | Phase::Peddlers
                | Phase::CitySales
                | Phase::Purchases => game.active_seat == Some(player.seat),
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

/// Sends the pending rebels pawn against a rival's peddler or caravan, if there is one.
async fn attack(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &PlayerRecord,
) -> Result<(), AppError> {
    let targets: Vec<(i64, bool)> = pending_fight(conn, game.id)
        .await?
        .map(|pending| pending.targets)
        .unwrap_or_default()
        .iter()
        .map(|pawn| (pawn.id, pawn.player_id == player.id))
        .collect();
    let target = choose_fight_target(&targets, &mut rand::rng()).ok_or(AppError::WrongPhase)?;
    record_rebel_attack(conn, game, player, target).await
}

/// Moves the AI's pawns toward the best buyers, or home.
async fn move_pawns(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &mut PlayerRecord,
) -> Result<(), AppError> {
    let movable: Vec<MovablePawn> = PawnRecord::list_for_game(conn, game.id)
        .await?
        .into_iter()
        .filter(|pawn| pawn.player_id == player.id && is_movable(pawn, game.turn, &player.stock))
        .map(|pawn| MovablePawn {
            id: pawn.id,
            kind: pawn.kind,
            location: pawn.location,
            item: pawn.item,
        })
        .collect();
    let board = load_board_state(conn, game.id).await?;
    let orders = choose_pawn_moves(&board, &movable, player.stock);
    record_pawn_moves(conn, game, player, &orders).await
}

/// Sells every item the AI's pawns can sell where they stand.
async fn sell(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &mut PlayerRecord,
) -> Result<(), AppError> {
    let board = load_board_state(conn, game.id).await?;
    let mut sold = SaleRecord::sites_sold(conn, game.id, game.turn).await?;
    let pawns = PawnRecord::list_for_game(conn, game.id).await?;
    let mut sellers = Vec::new();
    for pawn in pawns.iter().filter(|pawn| pawn.player_id == player.id) {
        if sale_offer(pawn, &board, &sold).is_none() {
            continue;
        }
        sellers.push(pawn.id);
        let site = pawn.location.site().and_then(site_by_id);
        if let Some(site) = site.filter(|site| !buys_without_limit(site)) {
            sold.push(site.id);
        }
    }
    record_sales(conn, game, player, &sellers).await
}

/// Buys at most one pawn or laboratory.
async fn buy(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &mut PlayerRecord,
) -> Result<(), AppError> {
    let players = PlayerRecord::list_for_game(conn, game.id).await?;
    let pawns = PawnRecord::list_for_game(conn, game.id).await?;
    let holdings = holdings_of(game, player, &players, &pawns);
    let purchase = choose_purchase(&holdings, player.laboratories);
    record_purchase(conn, game, player, purchase).await
}
