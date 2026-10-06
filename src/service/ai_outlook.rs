use sqlx::SqliteConnection;

use crate::db::{FanaticRecord, GameRecord, PawnRecord, PlayerRecord, SaleRecord};
use crate::domain::ai_trading::MovablePawn;
use crate::domain::strong_ai::{AiOutlook, RivalPawn};
use crate::domain::{Phase, Wealth};
use crate::error::AppError;
use crate::service::board_loader::load_board_state;

/// Gathers what `player`, a computer-controlled alchemist, knows of `game`.
///
/// Rivals' pawns come without their loads, and rivals' fanatics only as one unseen pool.
///
/// # Errors
/// Fails on a database error.
pub(crate) async fn load_outlook(
    conn: &mut SqliteConnection,
    game: &GameRecord,
    player: &PlayerRecord,
) -> Result<AiOutlook, AppError> {
    let players = PlayerRecord::list_for_game(conn, game.id).await?;
    let fanatics = FanaticRecord::list_for_game(conn, game.id).await?;
    let pawns = PawnRecord::list_for_game(conn, game.id).await?;
    let board = load_board_state(conn, game.id).await?;
    let sold_sites = SaleRecord::sites_sold(conn, game.id, game.turn).await?;
    let (own_fanatics, others_fanatics): (Vec<_>, Vec<_>) = fanatics
        .into_iter()
        .partition(|record| record.player_id == player.id);
    let rivals_with_fanatics = players
        .iter()
        .filter(|other| other.id != player.id)
        .filter(|other| {
            others_fanatics
                .iter()
                .any(|record| record.player_id == other.id && !record.in_hall)
        })
        .count();
    Ok(AiOutlook {
        turn: game.turn,
        max_turns: game.max_turns,
        sale_turns: sale_turns(game),
        board,
        money: player.money,
        laboratories: player.laboratories,
        total_laboratories: players.iter().map(|other| other.laboratories).sum(),
        stock: player.stock,
        supply: game.supply,
        hand: own_fanatics
            .iter()
            .filter(|record| !record.in_hall)
            .map(|record| record.fanatic)
            .collect(),
        own_in_hall: own_fanatics
            .iter()
            .find(|record| record.in_hall)
            .map(|record| record.fanatic),
        others_in_hall: others_fanatics
            .iter()
            .filter(|record| record.in_hall && !record.revealed)
            .count(),
        // Discarded pawns were revealed when played, so the fanatics still in the game that
        // are neither the AI's nor face up are exactly the full pool minus its own pawns and
        // every pawn revealed so far: a count any player can keep.
        unseen: others_fanatics
            .iter()
            .filter(|record| !record.revealed)
            .map(|record| record.fanatic)
            .collect(),
        rivals_with_fanatics,
        pawns: own_pawns(&pawns, player.id),
        rivals: rival_pawns(&pawns, &players, player.id),
        sold_sites,
    })
}

/// Phases V left in the game, this turn's included while it is still ahead.
fn sale_turns(game: &GameRecord) -> i64 {
    let this_turn = match game.phase {
        Phase::Announcements
        | Phase::Expenses
        | Phase::Conversions
        | Phase::RebelMovement
        | Phase::RebelFights
        | Phase::Production
        | Phase::Peddlers => 1,
        Phase::CitySales | Phase::Purchases | Phase::Finished => 0,
    };
    (game.max_turns - game.turn + this_turn).max(0)
}

fn own_pawns(pawns: &[PawnRecord], player_id: i64) -> Vec<MovablePawn> {
    pawns
        .iter()
        .filter(|pawn| pawn.player_id == player_id)
        .map(|pawn| MovablePawn {
            id: pawn.id,
            kind: pawn.kind,
            location: pawn.location,
            item: pawn.item,
        })
        .collect()
}

fn rival_pawns(pawns: &[PawnRecord], players: &[PlayerRecord], player_id: i64) -> Vec<RivalPawn> {
    pawns
        .iter()
        .filter(|pawn| pawn.player_id != player_id)
        .filter_map(|pawn| {
            let owner = players.iter().find(|other| other.id == pawn.player_id)?;
            Some(RivalPawn {
                location: pawn.location,
                kind: pawn.kind,
                owner_wealth: Wealth::of(owner.money),
            })
        })
        .collect()
}
