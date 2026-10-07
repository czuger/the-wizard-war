use sqlx::SqliteConnection;

use crate::db::{FanaticRecord, GameRecord, PawnRecord, PlayerRecord, RebelRecord};
use crate::domain::Phase;
use crate::domain::ai_trading::MovablePawn;
use crate::domain::strong_ai::{
    AiOutlook, FightCandidate, Rebel, choose_announcement, choose_city_sale, choose_conversion,
    choose_expense, choose_fight_target, choose_pawn_moves, choose_production, choose_purchase,
    choose_rebel_moves,
};
use crate::domain::{Conversion, ItemCounts, PawnOrder, Wealth};
use crate::error::AppError;
use crate::service::ai_outlook::load_outlook;
use crate::service::announce_fanatic::record_announcement;
use crate::service::city_sales::record_city_sale;
use crate::service::convert_site::record_conversion;
use crate::service::move_pawns::record_pawn_moves;
use crate::service::move_rebels::{RebelMove, record_rebel_moves};
use crate::service::pawn_rules::is_movable;
use crate::service::pay_expense::record_expense;
use crate::service::produce_items::{record_production, required_items};
use crate::service::purchases::{holdings_of, record_purchase};
use crate::service::rebel_fights::{pending_fight, record_rebel_attack};

/// Plays one decision of a « Fort » alchemist in the current phase. Sales of phase V are
/// left to the caller, as they are the same for every level.
///
/// # Errors
/// Fails on a database error, or if a decision is refused, which would be a bug.
pub(crate) async fn play_strong(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &mut PlayerRecord,
) -> Result<(), AppError> {
    let outlook = load_outlook(conn, game, player).await?;
    match game.phase {
        Phase::Announcements => announce(conn, game, player, outlook).await,
        Phase::Expenses => {
            let amount = choose_expense(&outlook);
            record_expense(conn, game, player, amount).await
        }
        Phase::Conversions => {
            let conversion = pick_conversion(&outlook);
            record_conversion(conn, game, player, conversion).await
        }
        Phase::RebelMovement => move_rebels(conn, game, player, &outlook).await,
        Phase::RebelFights => attack(conn, game, player).await,
        Phase::Production => {
            let count = required_items(player.laboratories, &game.supply);
            let items = pick_production(&outlook, count);
            record_production(conn, game, player, items).await
        }
        Phase::Peddlers => move_pawns(conn, game, player, &outlook).await,
        Phase::CitySales => {
            let item = choose_city_sale(&outlook);
            record_city_sale(conn, game, player, item).await
        }
        Phase::Purchases => {
            let players = PlayerRecord::list_for_game(conn, game.id).await?;
            let pawns = PawnRecord::list_for_game(conn, game.id).await?;
            let holdings = holdings_of(game, player, &players, &pawns);
            let purchase = choose_purchase(&outlook, &holdings);
            record_purchase(conn, game, player, purchase).await
        }
        Phase::Finished => Ok(()),
    }
}

/// Kept synchronous: the thread-local RNG must not live across an `.await`.
fn pick_conversion(outlook: &AiOutlook) -> Option<Conversion> {
    choose_conversion(outlook, &mut rand::rng())
}

/// Kept synchronous: the thread-local RNG must not live across an `.await`.
fn pick_production(outlook: &AiOutlook, count: i64) -> ItemCounts {
    choose_production(outlook, count, &mut rand::rng())
}

/// Kept synchronous: the thread-local RNG must not live across an `.await`.
fn pick_announcement(outlook: &AiOutlook) -> Option<usize> {
    choose_announcement(outlook, &mut rand::rng())
}

/// Kept synchronous: the thread-local RNG must not live across an `.await`.
fn pick_pawn_moves(outlook: &AiOutlook, movable: &[MovablePawn]) -> Vec<PawnOrder> {
    choose_pawn_moves(outlook, movable, &mut rand::rng())
}

/// Sends the fanatic [`choose_announcement`] picks, if any.
async fn announce(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &mut PlayerRecord,
    mut outlook: AiOutlook,
) -> Result<(), AppError> {
    let available: Vec<FanaticRecord> = FanaticRecord::list_for_player(conn, player.id)
        .await?
        .into_iter()
        .filter(|record| !record.in_hall)
        .collect();
    outlook.hand = available.iter().map(|record| record.fanatic).collect();
    let fanatic_id = pick_announcement(&outlook)
        .and_then(|index| available.get(index))
        .map(|record| record.id);
    record_announcement(conn, game, player, fanatic_id).await
}

/// Moves every rebels pawn where [`choose_rebel_moves`] sends it.
async fn move_rebels(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &PlayerRecord,
    outlook: &AiOutlook,
) -> Result<(), AppError> {
    let rebels: Vec<Rebel> = RebelRecord::list_for_game(conn, game.id)
        .await?
        .iter()
        .map(|record| Rebel {
            id: record.id,
            site: record.site,
        })
        .collect();
    let moves = pick_rebel_moves(outlook, &rebels);
    record_rebel_moves(conn, game, player, &moves).await
}

/// Kept synchronous: the thread-local RNG must not live across an `.await`.
fn pick_rebel_moves(outlook: &AiOutlook, rebels: &[Rebel]) -> Vec<RebelMove> {
    choose_rebel_moves(outlook, rebels, &mut rand::rng())
        .into_iter()
        .map(|(rebel_id, to)| RebelMove { rebel_id, to })
        .collect()
}

/// Sends the pending rebels pawn against the pawn [`choose_fight_target`] picks.
async fn attack(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &PlayerRecord,
) -> Result<(), AppError> {
    let players = PlayerRecord::list_for_game(conn, game.id).await?;
    let targets = pending_fight(conn, game.id)
        .await?
        .map(|pending| pending.targets)
        .unwrap_or_default();
    let candidates: Vec<FightCandidate> = targets
        .iter()
        .map(|pawn| FightCandidate {
            pawn_id: pawn.id,
            kind: pawn.kind,
            rival_wealth: players
                .iter()
                .find(|owner| owner.id == pawn.player_id && owner.id != player.id)
                .map(|owner| Wealth::of(owner.money)),
        })
        .collect();
    let target = choose_fight_target(&candidates).ok_or(AppError::WrongPhase)?;
    record_rebel_attack(conn, game, player, target).await
}

/// Moves the AI's pawns as [`choose_pawn_moves`] decides.
async fn move_pawns(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &mut PlayerRecord,
    outlook: &AiOutlook,
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
    let orders = pick_pawn_moves(outlook, &movable);
    record_pawn_moves(conn, game, player, &orders).await
}
