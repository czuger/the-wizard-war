use std::collections::BTreeSet;

use sqlx::{SqliteConnection, SqlitePool};

use crate::db::{EventRecord, GameRecord, PawnRecord, PlayerRecord, RebelRecord};
use crate::domain::board::site_by_id;
use crate::domain::rebel_movement::village_destinations;
use crate::domain::{Location, LogSection, Phase, SiteId};
use crate::error::AppError;
use crate::service::player_action::PlayerAction;
use crate::service::rebel_fights::continue_rebel_fights;
use crate::service::write_transaction::begin_write;

/// Where the first player sends one rebels pawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RebelMove {
    /// The pawn moved.
    pub rebel_id: i64,
    /// The adjacent village it goes to.
    pub to: SiteId,
}

/// Phase II.2: the turn's first player moves every rebels pawn one village along a path.
///
/// Rebels reaching peddlers or caravans then fight them; production (phase III) follows.
///
/// # Arguments
/// * `moves` - one move for each pawn that can move; pawns with no village nearby stay.
///
/// # Errors
/// Fails when it is not the rebel movement phase or the player is not the turn's first
/// player, when a pawn that can move has no valid move or a move is given twice or for an
/// unknown pawn, or on a database error.
pub async fn move_rebels(
    pool: &SqlitePool,
    game_id: i64,
    token: &str,
    moves: Vec<RebelMove>,
) -> Result<(), AppError> {
    let mut tx = begin_write(pool).await?;
    let PlayerAction { mut game, player } =
        PlayerAction::load(&mut tx, game_id, token, Phase::RebelMovement).await?;
    record_rebel_moves(&mut tx, &mut game, &player, &moves).await?;
    PlayerAction::finish(tx, game).await
}

/// Starts phase II.2 when a rebels pawn can move, otherwise goes straight to production.
///
/// The caller saves `game`.
pub(crate) async fn start_rebel_movement(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
) -> Result<(), AppError> {
    let rebels = RebelRecord::list_for_game(conn, game.id).await?;
    let any_can_move = rebels
        .iter()
        .any(|rebel| !village_destinations(rebel.site).is_empty());
    game.phase = if any_can_move {
        Phase::RebelMovement
    } else {
        Phase::Production
    };
    game.active_seat = Some(game.first_seat);
    Ok(())
}

/// Records the rebel moves of the first player of a game in the rebel movement phase, then
/// starts the fights, or production when there is none.
///
/// The caller saves `game`.
pub(crate) async fn record_rebel_moves(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &PlayerRecord,
    moves: &[RebelMove],
) -> Result<(), AppError> {
    if game.active_seat != Some(player.seat) {
        return Err(AppError::NotYourTurn);
    }
    let rebels = RebelRecord::list_for_game(conn, game.id).await?;
    check_moves(&rebels, moves)?;
    let pawns = PawnRecord::list_for_game(conn, game.id).await?;
    for rebel_move in moves {
        let from = rebels
            .iter()
            .find(|rebel| rebel.id == rebel_move.rebel_id)
            .map(|rebel| rebel.site);
        RebelRecord::move_to(conn, rebel_move.rebel_id, rebel_move.to).await?;
        let message = format!(
            "Des rebelles marchent de {} vers {}.",
            site_name(from),
            site_name(Some(rebel_move.to))
        );
        EventRecord::insert(conn, game.id, game.turn, LogSection::Conversions, &message).await?;
        let meets_pawns = pawns
            .iter()
            .any(|pawn| pawn.location == Location::Site(rebel_move.to));
        if meets_pawns {
            RebelRecord::set_must_fight(conn, rebel_move.rebel_id, true).await?;
        }
    }
    continue_rebel_fights(conn, game).await
}

/// Checks that every pawn able to move gets exactly one move, to an adjacent village.
fn check_moves(rebels: &[RebelRecord], moves: &[RebelMove]) -> Result<(), AppError> {
    let mut moved = BTreeSet::new();
    for rebel_move in moves {
        let rebel = rebels
            .iter()
            .find(|rebel| rebel.id == rebel_move.rebel_id)
            .ok_or(AppError::InvalidRebelMove)?;
        let valid = village_destinations(rebel.site).contains(&rebel_move.to);
        if !valid || !moved.insert(rebel.id) {
            return Err(AppError::InvalidRebelMove);
        }
    }
    let all_moved = rebels
        .iter()
        .all(|rebel| moved.contains(&rebel.id) || village_destinations(rebel.site).is_empty());
    if all_moved {
        Ok(())
    } else {
        Err(AppError::InvalidRebelMove)
    }
}

fn site_name(site: Option<SiteId>) -> &'static str {
    site.and_then(site_by_id).map_or("?", |site| site.name)
}
