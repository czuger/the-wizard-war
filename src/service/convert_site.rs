use sqlx::{SqliteConnection, SqlitePool};

use crate::db::{CultMarkerRecord, EventRecord, GameRecord, PlayerRecord, RebelRecord};
use crate::domain::{Conversion, LogSection, Phase, TurnOrder};
use crate::error::AppError;
use crate::service::board_loader::load_board_state;
use crate::service::move_rebels::start_rebel_movement;
use crate::service::player_action::PlayerAction;
use crate::service::write_transaction::begin_write;

/// Phase II.1: the active player places, flips or replaces one cult marker.
///
/// After the last player of the turn, the rebels move (phase II.2).
///
/// # Arguments
/// * `conversion` - the chosen action, or `None` to pass when no conversion is possible.
///
/// # Errors
/// Fails when it is not the conversions phase or not the player's turn, when the conversion
/// breaks a rule, when the player passes although a conversion is possible, or on a
/// database error.
pub async fn convert_site(
    pool: &SqlitePool,
    game_id: i64,
    token: &str,
    conversion: Option<Conversion>,
) -> Result<(), AppError> {
    let mut tx = begin_write(pool).await?;
    let PlayerAction { mut game, player } =
        PlayerAction::load(&mut tx, game_id, token, Phase::Conversions).await?;
    record_conversion(&mut tx, &mut game, &player, conversion).await?;
    PlayerAction::finish(tx, game).await
}

/// Records the conversion of the active player of a game in the conversions phase, and
/// starts the rebel movement after the last one.
///
/// The caller saves `game`.
pub(crate) async fn record_conversion(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &PlayerRecord,
    conversion: Option<Conversion>,
) -> Result<(), AppError> {
    if game.active_seat != Some(player.seat) {
        return Err(AppError::NotYourTurn);
    }
    let board = load_board_state(conn, game.id).await?;
    let message = match conversion {
        Some(conversion) => {
            let marker = board.validate(&conversion, game.turn)?;
            let site = conversion.site();
            CultMarkerRecord { site, marker }
                .save(conn, game.id)
                .await?;
            if let Conversion::Replace { .. } = conversion {
                RebelRecord::insert(conn, game.id, site).await?;
            }
            conversion.narrate(&player.name, marker)
        }
        None if board.legal_conversions(game.turn).is_empty() => {
            format!("{} ne peut convertir aucun lieu.", player.name)
        }
        None => return Err(AppError::ConversionRequired),
    };
    EventRecord::insert(conn, game.id, game.turn, LogSection::Conversions, &message).await?;

    let order = TurnOrder::new(game.player_count, game.first_seat);
    game.active_seat = order.next_after(player.seat);
    if game.active_seat.is_none() {
        start_rebel_movement(conn, game).await?;
    }
    Ok(())
}
