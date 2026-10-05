use sqlx::{SqliteConnection, SqlitePool};

use crate::db::{EventRecord, GameRecord, PawnRecord, PlayerRecord};
use crate::domain::rules::{BANK_CLOSED_TURNS, LABORATORY_COUNT};
use crate::domain::{Holdings, Location, PawnKind, Phase, Purchase};
use crate::error::AppError;
use crate::service::end_turn::end_turn;
use crate::service::player_action::PlayerAction;
use crate::service::seat_cycle::next_eligible_seat;
use crate::service::write_transaction::begin_write;

/// Starts phase VII with the first player of the turn who can buy or sell something, or
/// ends the turn when nobody can.
///
/// The caller saves `game`.
pub(crate) async fn start_purchases(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
) -> Result<(), AppError> {
    game.phase = Phase::Purchases;
    advance(conn, game, None).await
}

/// Phase VII: the active player buys peddlers, caravans and laboratories from the bank, and
/// may sell back peddlers and caravans in the city at half price, except in the last turns.
///
/// # Errors
/// Fails when it is not phase VII or not the player's turn, when the purchase breaks a rule,
/// or on a database error.
pub async fn buy_from_bank(
    pool: &SqlitePool,
    game_id: i64,
    token: &str,
    purchase: Purchase,
) -> Result<(), AppError> {
    let mut tx = begin_write(pool).await?;
    let PlayerAction {
        mut game,
        mut player,
    } = PlayerAction::load(&mut tx, game_id, token, Phase::Purchases).await?;
    record_purchase(&mut tx, &mut game, &mut player, purchase).await?;
    PlayerAction::finish(tx, game).await
}

/// Records the purchase of the active player of a game in phase VII, then hands over.
///
/// The player is saved; the caller saves `game`.
pub(crate) async fn record_purchase(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    player: &mut PlayerRecord,
    purchase: Purchase,
) -> Result<(), AppError> {
    if game.active_seat != Some(player.seat) {
        return Err(AppError::NotYourTurn);
    }
    let players = PlayerRecord::list_for_game(conn, game.id).await?;
    let pawns = PawnRecord::list_for_game(conn, game.id).await?;
    purchase.check(&holdings_of(game, player, &players, &pawns))?;

    let player_id = player.id;
    let in_city = |kind: PawnKind| -> Vec<i64> {
        pawns
            .iter()
            .filter(|pawn| {
                pawn.player_id == player_id && pawn.kind == kind && pawn.location == Location::City
            })
            .map(|pawn| pawn.id)
            .collect()
    };
    let sold: Vec<i64> = in_city(PawnKind::Peddler)
        .into_iter()
        .take(purchase.sold_peddlers as usize)
        .chain(
            in_city(PawnKind::Caravan)
                .into_iter()
                .take(purchase.sold_caravans as usize),
        )
        .collect();
    for id in sold {
        PawnRecord::delete(conn, id).await?;
    }
    for (kind, count) in [
        (PawnKind::Peddler, purchase.peddlers),
        (PawnKind::Caravan, purchase.caravans),
    ] {
        for _ in 0..count {
            PawnRecord::insert(conn, game.id, player.id, kind).await?;
        }
    }
    player.laboratories += purchase.laboratories;
    player.money += purchase.refund() - purchase.cost();
    player.save(conn).await?;
    let message = describe(&player.name, &purchase);
    EventRecord::insert(conn, game.id, game.turn, &message).await?;
    advance(conn, game, Some(player.seat)).await
}

/// What a player owns, as phase VII sees it.
pub(crate) fn holdings_of(
    game: &GameRecord,
    player: &PlayerRecord,
    players: &[PlayerRecord],
    pawns: &[PawnRecord],
) -> Holdings {
    let count = |kind: PawnKind, city_only: bool| {
        pawns
            .iter()
            .filter(|pawn| pawn.player_id == player.id && pawn.kind == kind)
            .filter(|pawn| !city_only || pawn.location == Location::City)
            .count() as i64
    };
    let laboratories: i64 = players.iter().map(|other| other.laboratories).sum();
    Holdings {
        money: player.money,
        peddlers: count(PawnKind::Peddler, false),
        caravans: count(PawnKind::Caravan, false),
        peddlers_in_city: count(PawnKind::Peddler, true),
        caravans_in_city: count(PawnKind::Caravan, true),
        laboratories_left: LABORATORY_COUNT - laboratories,
        bank_open: is_bank_open(game),
    }
}

/// Whether the bank still buys pawns back: it closes for the last two turns.
pub(crate) fn is_bank_open(game: &GameRecord) -> bool {
    game.turn <= game.max_turns - BANK_CLOSED_TURNS
}

/// Whether some purchase or buy-back is possible with these holdings.
pub(crate) fn can_trade(holdings: &Holdings) -> bool {
    let single = [
        Purchase {
            peddlers: 1,
            ..Purchase::default()
        },
        Purchase {
            caravans: 1,
            ..Purchase::default()
        },
        Purchase {
            laboratories: 1,
            ..Purchase::default()
        },
        Purchase {
            sold_peddlers: 1,
            ..Purchase::default()
        },
        Purchase {
            sold_caravans: 1,
            ..Purchase::default()
        },
    ];
    single
        .iter()
        .any(|purchase| purchase.check(holdings).is_ok())
}

/// Hands phase VII to the next player who can trade, or ends the turn.
async fn advance(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    after: Option<i64>,
) -> Result<(), AppError> {
    let mut players = PlayerRecord::list_for_game(conn, game.id).await?;
    let pawns = PawnRecord::list_for_game(conn, game.id).await?;
    let can_play = |seat: i64| {
        players
            .iter()
            .find(|player| player.seat == seat)
            .is_some_and(|player| can_trade(&holdings_of(game, player, &players, &pawns)))
    };
    match next_eligible_seat(game, after, can_play) {
        Some(seat) => {
            game.active_seat = Some(seat);
            Ok(())
        }
        None => end_turn(conn, game, &mut players).await,
    }
}

/// French log line for a purchase.
fn describe(name: &str, purchase: &Purchase) -> String {
    let parts: Vec<String> = [
        (purchase.peddlers, "achète", "colporteur(s)"),
        (purchase.caravans, "achète", "caravane(s)"),
        (purchase.laboratories, "achète", "laboratoire(s)"),
        (purchase.sold_peddlers, "revend", "colporteur(s)"),
        (purchase.sold_caravans, "revend", "caravane(s)"),
    ]
    .into_iter()
    .filter(|(count, _, _)| *count > 0)
    .map(|(count, verb, what)| format!("{verb} {count} {what}"))
    .collect();
    if parts.is_empty() {
        format!("{name} n'achète rien.")
    } else {
        format!("{name} {}.", parts.join(", "))
    }
}
