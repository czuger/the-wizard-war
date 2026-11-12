//! Writes the per-game AI log: each AI decision with the hidden state it was taken in and
//! its reasons, and the whole table's hidden state at the end of each turn.

use sqlx::{SqliteConnection, SqlitePool};

use crate::db::{AiLogRecord, FanaticRecord, GameRecord, PawnRecord, PlayerRecord};
use crate::domain::{DecisionTrace, Phase};
use crate::error::AppError;

/// Where and when an AI decision was taken, captured before it changes the game.
pub(crate) struct DecisionContext {
    turn: i64,
    phase: Phase,
    snapshot: String,
}

impl DecisionContext {
    /// Captures the turn, the phase and the hidden state of `player` before it decides.
    pub(crate) async fn capture(
        conn: &mut SqliteConnection,
        game: &GameRecord,
        player: &PlayerRecord,
    ) -> Result<DecisionContext, AppError> {
        Ok(DecisionContext {
            turn: game.turn,
            phase: game.phase,
            snapshot: player_snapshot(conn, game, player).await?,
        })
    }

    /// Writes the decision of `player`, explained by `trace`, to the game's AI log.
    pub(crate) async fn record(
        self,
        conn: &mut SqliteConnection,
        game: &GameRecord,
        player: &PlayerRecord,
        trace: &DecisionTrace,
    ) -> Result<(), AppError> {
        let mut body = format!(
            "{} (seat {}, {:?} AI, {:?})\n{}",
            player.name,
            player.seat,
            player.ai_level.unwrap_or(game.ai_level),
            player.temperament(),
            self.snapshot
        );
        for line in trace.lines() {
            body.push('\n');
            body.push_str(line);
        }
        AiLogRecord::insert(conn, game.id, self.turn, self.phase, Some(player.id), &body).await
    }
}

/// Writes every player's hidden state, the human's included, at the end of the turn.
///
/// # Errors
/// A database error.
pub(crate) async fn record_turn_summary(
    conn: &mut SqliteConnection,
    game: &GameRecord,
    players: &[PlayerRecord],
) -> Result<(), AppError> {
    let mut body = format!(
        "End of turn {} of {}; supply {:?}; collective expense {}",
        game.turn, game.max_turns, game.supply, game.collective_expense
    );
    for player in players {
        let kind = if player.is_ai { "AI" } else { "human" };
        let snapshot = player_snapshot(conn, game, player).await?;
        body.push_str(&format!(
            "\n{} (seat {}, {kind})\n{snapshot}",
            player.name, player.seat
        ));
    }
    AiLogRecord::insert(conn, game.id, game.turn, game.phase, None, &body).await
}

/// The money, laboratories, items, fanatics and pawns of `player`, indented.
async fn player_snapshot(
    conn: &mut SqliteConnection,
    game: &GameRecord,
    player: &PlayerRecord,
) -> Result<String, AppError> {
    let fanatics = FanaticRecord::list_for_player(conn, player.id).await?;
    let hand: Vec<String> = fanatics
        .iter()
        .map(|record| {
            let place = if record.in_hall { " in hall" } else { "" };
            format!(
                "{:?} {}{place}",
                record.fanatic.cult, record.fanatic.strength
            )
        })
        .collect();
    let pawns: Vec<String> = PawnRecord::list_for_game(conn, game.id)
        .await?
        .iter()
        .filter(|pawn| pawn.player_id == player.id)
        .map(|pawn| {
            let stopped = if pawn.stopped_until_turn >= game.turn {
                format!(", stopped until turn {}", pawn.stopped_until_turn)
            } else {
                String::new()
            };
            format!(
                "#{} {:?} at {:?} carrying {:?}{stopped}",
                pawn.id, pawn.kind, pawn.location, pawn.item
            )
        })
        .collect();
    Ok(format!(
        "  money {}, laboratories {}, stock {:?}, production {:?}, expense {}\n  fanatics [{}]\n  pawns [{}]",
        player.money,
        player.laboratories,
        player.stock,
        player.production,
        player.turn_expense,
        hand.join(", "),
        pawns.join("; ")
    ))
}

/// Returns a game's whole AI log as plain text, oldest entry first, one block per entry.
///
/// # Errors
/// [`AppError::GameNotFound`] when the game does not exist, or a database error.
pub async fn export_ai_log(pool: &SqlitePool, game_id: i64) -> Result<String, AppError> {
    let mut conn = pool.acquire().await?;
    let game = GameRecord::find(&mut conn, game_id).await?;
    let entries = AiLogRecord::list_for_game(&mut conn, game_id).await?;
    let mut text = format!(
        "AI log of game {} ({:?} AI, {} players, {} turns, now {:?} of turn {})\n",
        game.id, game.ai_level, game.player_count, game.max_turns, game.phase, game.turn
    );
    for entry in entries {
        let actor = entry.player_name.as_deref().unwrap_or("table");
        text.push_str(&format!(
            "\n=== turn {} · {} · {actor} · {}\n{}\n",
            entry.turn, entry.phase, entry.created_at, entry.body
        ));
    }
    Ok(text)
}
