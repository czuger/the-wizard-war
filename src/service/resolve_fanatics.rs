use sqlx::SqliteConnection;

use crate::db::{EventRecord, FanaticRecord, GameRecord, PlayerRecord};
use crate::domain::{Cult, Fanatic, FanaticVerdict, dominant_cult};
use crate::error::AppError;
use crate::service::seat_cycle::seats_in_turn_order;

/// Phase IV: reveals the great hall, applies the Dominant Fanatic to every laboratory,
/// and moves the surviving production into stock. The hall's pawns stay face up until
/// [`end_turn`](crate::service::end_turn::end_turn) discards them.
///
/// Players are handled in turn order, which matters when the supply runs out of bonus items.
/// The records are updated in place; the caller saves `game`.
pub async fn resolve_fanatics(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    players: &mut [PlayerRecord],
) -> Result<(), AppError> {
    let hall: Vec<Fanatic> = FanaticRecord::list_in_hall(conn, game.id)
        .await?
        .into_iter()
        .map(|record| record.fanatic)
        .collect();
    let dominant = dominant_cult(&hall);
    EventRecord::insert(conn, game.id, game.turn, &hall_message(&hall, dominant)).await?;

    for seat in seats_in_turn_order(game) {
        let Some(player) = players.iter_mut().find(|player| player.seat == seat) else {
            continue;
        };
        let verdict = dominant
            .map(|cult| FanaticVerdict::judge(cult, &player.production))
            .unwrap_or(FanaticVerdict::Unaffected);
        let bonus = verdict
            .bonus_item()
            .filter(|item| game.supply.get(*item) > 0);
        if let Some(item) = bonus {
            game.supply = game.supply.with(item, game.supply.get(item) - 1);
        }
        player.stock = verdict.new_stock(player.production, player.stock, bonus.is_some());
        player.production = Default::default();
        player.save(conn).await?;
        if dominant.is_some() {
            let message = verdict.describe(&player.name, bonus.is_some());
            EventRecord::insert(conn, game.id, game.turn, &message).await?;
        }
    }
    FanaticRecord::reveal_hall(conn, game.id).await
}

fn hall_message(hall: &[Fanatic], dominant: Option<Cult>) -> String {
    if hall.is_empty() {
        return "Personne n'a envoyé de fanatique : les laboratoires sont tranquilles.".to_string();
    }
    let revealed: Vec<String> = hall.iter().map(Fanatic::label).collect();
    let outcome = match dominant {
        Some(cult) => format!("la Dominante Fanatique est {}.", cult.label()),
        _ => "égalité, les fanatiques ne font rien.".to_string(),
    };
    format!("Grande salle : {} — {outcome}", revealed.join(", "))
}
