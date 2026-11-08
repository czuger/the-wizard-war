use sqlx::SqliteConnection;

use crate::db::{FanaticRecord, GameRecord, PawnRecord, PlayerRecord, RebelRecord};
use crate::domain::Phase;
use crate::domain::ai_trading::MovablePawn;
use crate::domain::strong_ai::{
    AiOutlook, FightCandidate, Rebel, choose_announcement, choose_city_sale, choose_conversion,
    choose_expense, choose_fight_target, choose_pawn_moves, choose_production, choose_purchase,
    choose_rebel_moves,
};
use crate::domain::{Conversion, DecisionTrace, ItemCounts, PawnOrder, Wealth};
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
    trace: &mut DecisionTrace,
) -> Result<(), AppError> {
    let outlook = load_outlook(conn, game, player).await?;
    note_outlook(&outlook, trace);
    match game.phase {
        Phase::Announcements => announce(conn, game, player, outlook, trace).await,
        Phase::Expenses => {
            let amount = choose_expense(&outlook);
            trace.note(format!("decision: pay {amount}"));
            record_expense(conn, game, player, amount).await
        }
        Phase::Conversions => {
            let conversion = pick_conversion(&outlook, trace);
            trace.note(format!("decision: {conversion:?}"));
            record_conversion(conn, game, player, conversion).await
        }
        Phase::RebelMovement => move_rebels(conn, game, player, &outlook, trace).await,
        Phase::RebelFights => attack(conn, game, player, trace).await,
        Phase::Production => {
            let count = required_items(player.laboratories, &game.supply);
            let items = pick_production(&outlook, count, trace);
            trace.note(format!("decision: make {items:?}"));
            record_production(conn, game, player, items).await
        }
        Phase::Peddlers => move_pawns(conn, game, player, &outlook, trace).await,
        Phase::CitySales => {
            let item = choose_city_sale(&outlook, trace);
            trace.note(format!("decision: sell {item:?}"));
            record_city_sale(conn, game, player, item).await
        }
        Phase::Purchases => {
            let players = PlayerRecord::list_for_game(conn, game.id).await?;
            let pawns = PawnRecord::list_for_game(conn, game.id).await?;
            let holdings = holdings_of(game, player, &players, &pawns);
            let purchase = choose_purchase(&outlook, &holdings);
            trace.note(format!("{holdings:?}; decision: {purchase:?}"));
            record_purchase(conn, game, player, purchase).await
        }
        Phase::Finished => Ok(()),
    }
}

/// Kept synchronous: the thread-local RNG must not live across an `.await`.
fn pick_conversion(outlook: &AiOutlook, trace: &mut DecisionTrace) -> Option<Conversion> {
    choose_conversion(outlook, &mut rand::rng(), trace)
}

/// Kept synchronous: the thread-local RNG must not live across an `.await`.
fn pick_production(outlook: &AiOutlook, count: i64, trace: &mut DecisionTrace) -> ItemCounts {
    choose_production(outlook, count, &mut rand::rng(), trace)
}

/// Kept synchronous: the thread-local RNG must not live across an `.await`.
fn pick_announcement(outlook: &AiOutlook, trace: &mut DecisionTrace) -> Option<usize> {
    choose_announcement(outlook, &mut rand::rng(), trace)
}

/// Kept synchronous: the thread-local RNG must not live across an `.await`.
fn pick_pawn_moves(
    outlook: &AiOutlook,
    movable: &[MovablePawn],
    trace: &mut DecisionTrace,
) -> Vec<PawnOrder> {
    choose_pawn_moves(outlook, movable, &mut rand::rng(), trace)
}

/// Sends the fanatic [`choose_announcement`] picks, if any.
async fn announce(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &mut PlayerRecord,
    mut outlook: AiOutlook,
    trace: &mut DecisionTrace,
) -> Result<(), AppError> {
    let available: Vec<FanaticRecord> = FanaticRecord::list_for_player(conn, player.id)
        .await?
        .into_iter()
        .filter(|record| !record.in_hall)
        .collect();
    outlook.hand = available.iter().map(|record| record.fanatic).collect();
    let fanatic_id = pick_announcement(&outlook, trace)
        .and_then(|index| available.get(index))
        .map(|record| record.id);
    trace.note(format!("decision: send fanatic #{fanatic_id:?}"));
    record_announcement(conn, game, player, fanatic_id).await
}

/// Moves every rebels pawn where [`choose_rebel_moves`] sends it.
async fn move_rebels(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &PlayerRecord,
    outlook: &AiOutlook,
    trace: &mut DecisionTrace,
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
    trace.note(format!("rebels {rebels:?}; decision: {moves:?}"));
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
    trace: &mut DecisionTrace,
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
    trace.note(format!(
        "candidates {candidates:?}; decision: attack pawn #{target}"
    ));
    record_rebel_attack(conn, game, player, target).await
}

/// Moves the AI's pawns as [`choose_pawn_moves`] decides.
async fn move_pawns(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &mut PlayerRecord,
    outlook: &AiOutlook,
    trace: &mut DecisionTrace,
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
    let orders = pick_pawn_moves(outlook, &movable, trace);
    trace.note(format!("decision: {orders:?}"));
    record_pawn_moves(conn, game, player, &orders).await
}

/// Notes what the AI believes before deciding: what its items are worth, and what it knows
/// of the fanatics and of its rivals.
fn note_outlook(outlook: &AiOutlook, trace: &mut DecisionTrace) {
    trace.note(format!(
        "outlook: turn {}/{}, {} sale turn(s) left, supply {:?}, {} laboratories in play",
        outlook.turn,
        outlook.max_turns,
        outlook.sale_turns,
        outlook.supply,
        outlook.total_laboratories
    ));
    trace.note(format!("item values: {:?}", outlook.item_values()));
    trace.note(format!(
        "fanatics: {} unseen, {} rival(s) holding some, {} rival(s) in hall, own in hall {:?}",
        outlook.unseen.len(),
        outlook.rivals_with_fanatics,
        outlook.others_in_hall,
        outlook.own_in_hall
    ));
    trace.note(format!(
        "rival pawns {:?}; sites sold this turn {:?}",
        outlook.rivals, outlook.sold_sites
    ));
}
