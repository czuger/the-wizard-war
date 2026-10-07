use rand::RngExt;
use sqlx::SqliteConnection;

use crate::db::{EventRecord, GameRecord, PawnRecord, RebelRecord};
use crate::domain::board::site_by_id;
use crate::domain::{FightOutcome, LogSection};
use crate::error::AppError;

/// Fights a rebels pawn against a peddler or caravan, then removes the rebels pawn.
///
/// The pawn is saved, or deleted when destroyed. A robbed pawn's item goes back to the
/// supply; the caller saves `game`.
///
/// # Arguments
/// * `owner` - name of the pawn's owner, for the log.
/// * `stopped_until_turn` - last turn an empty, robbed pawn may not move.
pub(crate) async fn fight(
    conn: &mut SqliteConnection,
    game: &mut GameRecord,
    mut pawn: PawnRecord,
    owner: &str,
    rebel: &RebelRecord,
    stopped_until_turn: i64,
) -> Result<(), AppError> {
    let die: u8 = rand::rng().random_range(1..=6);
    let outcome = FightOutcome::from_die(die);
    let result = match outcome {
        FightOutcome::CalmedDown => "la bagarre se calme",
        FightOutcome::Robbed => match pawn.item.take() {
            Some(item) => {
                game.supply = game.supply.with(item, game.supply.get(item) + 1);
                "l'objet transporté retourne à la banque"
            }
            None => {
                pawn.stopped_until_turn = stopped_until_turn;
                "le pion s'arrête un tour"
            }
        },
        FightOutcome::Destroyed => "le pion et son chargement sont éliminés",
    };
    if outcome == FightOutcome::Destroyed {
        PawnRecord::delete(conn, pawn.id).await?;
    } else {
        pawn.save(conn).await?;
    }
    RebelRecord::delete(conn, rebel.id).await?;
    let site = site_by_id(rebel.site).map_or("?", |site| site.name);
    let message = format!(
        "Les rebelles attaquent {} de {owner} à {site} (dé : {die}) : {result}. Les rebelles quittent le jeu.",
        pawn.kind.with_article()
    );
    EventRecord::insert(
        conn,
        game.id,
        game.turn,
        LogSection::of(game.phase),
        &message,
    )
    .await
}
